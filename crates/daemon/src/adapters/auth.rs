use anyhow::{Context, Result, anyhow};
use libc::{c_char, c_int, c_void};
use std::env;
use std::ffi::CString;
use std::ptr;

pub trait Authenticator: Send + Sync {
    fn verify_password(&self, password: &str) -> Result<bool>;
    fn username(&self) -> &str;
}

pub struct PamAuthenticator {
    service_name: String,
    username: String,
}

impl PamAuthenticator {
    pub fn from_env() -> Result<Self> {
        let username = env::var("KWYLOCK_PAM_USER")
            .or_else(|_| env::var("USER"))
            .or_else(|_| env::var("LOGNAME"))
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("unable to determine PAM username from environment"))?;

        Ok(Self {
            service_name: "login".to_string(),
            username,
        })
    }
}

impl Authenticator for PamAuthenticator {
    fn verify_password(&self, password: &str) -> Result<bool> {
        verify_with_pam(&self.service_name, &self.username, password)
    }

    fn username(&self) -> &str {
        &self.username
    }
}

fn verify_with_pam(service: &str, username: &str, password: &str) -> Result<bool> {
    let service = CString::new(service).context("PAM service contains interior NUL")?;
    let username = CString::new(username).context("PAM username contains interior NUL")?;
    let password = CString::new(password).context("password contains interior NUL")?;

    let mut pam_handle: *mut PamHandle = ptr::null_mut();
    let mut conversation_data = Box::new(ConversationData { password });

    let conversation = PamConversation {
        conv: Some(pam_conversation),
        appdata_ptr: (&mut *conversation_data as *mut ConversationData).cast(),
    };

    // SAFETY: Pointers are valid for the duration of PAM interaction and handle output is valid.
    let start_rc = unsafe {
        pam_start(
            service.as_ptr(),
            username.as_ptr(),
            &conversation,
            &mut pam_handle,
        )
    };
    if start_rc != PAM_SUCCESS {
        return Ok(false);
    }

    // SAFETY: pam_handle was initialized by pam_start and remains valid until pam_end.
    let auth_rc = unsafe { pam_authenticate(pam_handle, 0) };
    if auth_rc != PAM_SUCCESS {
        // SAFETY: pam_handle is valid because pam_start succeeded above.
        unsafe {
            pam_end(pam_handle, auth_rc);
        }
        return Ok(false);
    }

    // SAFETY: pam_handle was initialized by pam_start and remains valid until pam_end.
    let acct_rc = unsafe { pam_acct_mgmt(pam_handle, 0) };
    // SAFETY: pam_handle is valid because pam_start succeeded above.
    unsafe {
        pam_end(pam_handle, acct_rc);
    }

    Ok(acct_rc == PAM_SUCCESS)
}

#[repr(C)]
struct PamHandle {
    _private: [u8; 0],
}

#[repr(C)]
struct PamMessage {
    msg_style: c_int,
    msg: *const c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut c_char,
    resp_retcode: c_int,
}

#[repr(C)]
struct PamConversation {
    conv: Option<
        unsafe extern "C" fn(
            c_int,
            *mut *const PamMessage,
            *mut *mut PamResponse,
            *mut c_void,
        ) -> c_int,
    >,
    appdata_ptr: *mut c_void,
}

struct ConversationData {
    password: CString,
}

const PAM_SUCCESS: c_int = 0;
const PAM_BUF_ERR: c_int = 5;
const PAM_CONV_ERR: c_int = 19;
const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_PROMPT_ECHO_ON: c_int = 2;
const PAM_ERROR_MSG: c_int = 3;
const PAM_TEXT_INFO: c_int = 4;

unsafe extern "C" fn pam_conversation(
    num_msg: c_int,
    msg: *mut *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata_ptr: *mut c_void,
) -> c_int {
    if num_msg <= 0 || msg.is_null() || resp.is_null() || appdata_ptr.is_null() {
        return PAM_CONV_ERR;
    }

    // SAFETY: appdata_ptr is created from a live ConversationData in verify_with_pam.
    let data = unsafe { &*(appdata_ptr as *const ConversationData) };
    // SAFETY: calloc returns memory suitable for num_msg pam_response entries.
    let responses = unsafe {
        libc::calloc(num_msg as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse
    };
    if responses.is_null() {
        return PAM_BUF_ERR;
    }

    for index in 0..num_msg as isize {
        // SAFETY: msg points to an array of num_msg entries provided by PAM.
        let message_ptr = unsafe { *msg.offset(index) };
        if message_ptr.is_null() {
            // SAFETY: responses was allocated above and is safe to free here.
            unsafe {
                libc::free(responses.cast());
            }
            return PAM_CONV_ERR;
        }

        // SAFETY: message_ptr was checked for null and comes from PAM.
        let message = unsafe { &*message_ptr };
        // SAFETY: responses points to valid array entry allocated with calloc.
        let response = unsafe { &mut *responses.offset(index) };

        match message.msg_style {
            PAM_PROMPT_ECHO_OFF | PAM_PROMPT_ECHO_ON => {
                // SAFETY: data.password is a valid C string and strdup allocates owned memory for PAM.
                let duplicated = unsafe { libc::strdup(data.password.as_ptr()) };
                if duplicated.is_null() {
                    // SAFETY: responses was allocated above and is safe to free here.
                    unsafe {
                        libc::free(responses.cast());
                    }
                    return PAM_BUF_ERR;
                }
                response.resp = duplicated;
                response.resp_retcode = 0;
            }
            PAM_ERROR_MSG | PAM_TEXT_INFO => {
                response.resp = ptr::null_mut();
                response.resp_retcode = 0;
            }
            _ => {
                // SAFETY: responses was allocated above and is safe to free here.
                unsafe {
                    libc::free(responses.cast());
                }
                return PAM_CONV_ERR;
            }
        }
    }

    // SAFETY: resp is a valid out-pointer from PAM and responses was allocated for it.
    unsafe {
        *resp = responses;
    }
    PAM_SUCCESS
}

#[link(name = "pam")]
unsafe extern "C" {
    fn pam_start(
        service_name: *const c_char,
        user: *const c_char,
        pam_conversation: *const PamConversation,
        pamh: *mut *mut PamHandle,
    ) -> c_int;
    fn pam_end(pamh: *mut PamHandle, pam_status: c_int) -> c_int;
    fn pam_authenticate(pamh: *mut PamHandle, flags: c_int) -> c_int;
    fn pam_acct_mgmt(pamh: *mut PamHandle, flags: c_int) -> c_int;
}
