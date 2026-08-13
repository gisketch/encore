use super::model::PermissionState;

pub trait PermissionProvider: Send + Sync {
    fn current(&self) -> PermissionState;
    fn request(&self) -> PermissionState;
}

pub struct SystemPermission;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

impl PermissionProvider for SystemPermission {
    fn current(&self) -> PermissionState {
        #[cfg(target_os = "macos")]
        {
            if unsafe { CGPreflightScreenCaptureAccess() } {
                PermissionState::Granted
            } else {
                PermissionState::PermissionRequired
            }
        }
        #[cfg(not(target_os = "macos"))]
        PermissionState::PermissionDenied
    }

    fn request(&self) -> PermissionState {
        #[cfg(target_os = "macos")]
        {
            if unsafe { CGRequestScreenCaptureAccess() } {
                PermissionState::RestartRequired
            } else {
                PermissionState::PermissionDenied
            }
        }
        #[cfg(not(target_os = "macos"))]
        PermissionState::PermissionDenied
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakePermission(Mutex<Vec<PermissionState>>);

    impl PermissionProvider for FakePermission {
        fn current(&self) -> PermissionState {
            self.0.lock().unwrap()[0]
        }

        fn request(&self) -> PermissionState {
            self.0.lock().unwrap().remove(0)
        }
    }

    #[test]
    fn provider_is_injectable_without_touching_tcc() {
        let fake = FakePermission(Mutex::new(vec![PermissionState::RestartRequired]));
        assert_eq!(fake.request(), PermissionState::RestartRequired);
    }
}
