use windows::Win32::Foundation;
use winfsp::FspError;

macro_rules! ntstatus {
    ($err_name:ident, $err:expr) => {
        pub const $err_name: FspError = FspError::NTSTATUS($err.0);
    };
}

ntstatus!(STATUS_INVALID_PARAMETER, Foundation::STATUS_INVALID_PARAMETER);
ntstatus!(STATUS_OBJECT_NAME_NOT_FOUND, Foundation::STATUS_OBJECT_NAME_NOT_FOUND);
ntstatus!(STATUS_ACCESS_DENIED, Foundation::STATUS_ACCESS_DENIED);
ntstatus!(STATUS_IO_DEVICE_ERROR, Foundation::STATUS_IO_DEVICE_ERROR);
