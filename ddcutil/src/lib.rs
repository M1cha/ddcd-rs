use error_rules::*;
use std::convert::TryInto;

use ddcutil_sys::DDCA_Non_Table_Vcp_Value as VcpValue;
pub const DDCRC_OK: ddcutil_sys::DDCA_Status = ddcutil_sys::DDCRC_OK as ddcutil_sys::DDCA_Status;

#[derive(Debug, Error)]
pub enum Error {
    #[error_kind("DDCA_Status:{}", 0)]
    Sys(ddcutil_sys::DDCA_Status),
    #[error_from]
    TryFromIntError(std::num::TryFromIntError),
}

pub struct DisplayIdentifier {
    native: ddcutil_sys::DDCA_Display_Identifier,
}

impl Drop for DisplayIdentifier {
    fn drop(&mut self) {
        let status = unsafe { ddcutil_sys::ddca_free_display_identifier(self.native) };
        if status != DDCRC_OK {
            panic!("ddca_free_display_identifier failed: {}", status);
        }
    }
}

impl std::fmt::Display for DisplayIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = unsafe { ddcutil_sys::ddca_did_repr(self.native) };
        if s.is_null() {
            return Err(std::fmt::Error);
        }

        match unsafe { std::ffi::CStr::from_ptr(s) }.to_str() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl DisplayIdentifier {
    pub fn from_dispno(dispno: usize) -> Result<Self, Error> {
        let mut did = std::ptr::null_mut();
        let status = unsafe {
            ddcutil_sys::ddca_create_dispno_display_identifier(dispno.try_into()?, &mut did)
        };
        if status != DDCRC_OK {
            Err(Error::Sys(status))
        } else {
            assert!(!did.is_null());
            Ok(Self { native: did })
        }
    }

    pub fn get_display_ref(&self) -> Result<DisplayRef, Error> {
        let mut dref = std::ptr::null_mut();
        let status = unsafe { ddcutil_sys::ddca_get_display_ref(self.native, &mut dref) };
        if status != DDCRC_OK {
            Err(Error::Sys(status))
        } else {
            assert!(!dref.is_null());
            Ok(DisplayRef { native: dref })
        }
    }
}

pub struct DisplayRef {
    native: ddcutil_sys::DDCA_Display_Ref,
}

impl DisplayRef {
    pub fn open_display2(&self, wait: bool) -> Result<DisplayHandle, Error> {
        let mut dh = std::ptr::null_mut();
        let status = unsafe { ddcutil_sys::ddca_open_display2(self.native, wait, &mut dh) };
        if status != DDCRC_OK {
            Err(Error::Sys(status))
        } else {
            assert!(!dh.is_null());
            Ok(DisplayHandle { native: dh })
        }
    }
}

impl Drop for DisplayRef {
    fn drop(&mut self) {
        let status = unsafe { ddcutil_sys::ddca_free_display_ref(self.native) };
        if status != DDCRC_OK {
            panic!("ddca_free_display_ref failed: {}", status);
        }
    }
}

impl std::fmt::Display for DisplayRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = unsafe { ddcutil_sys::ddca_dref_repr(self.native) };
        if s.is_null() {
            return Err(std::fmt::Error);
        }

        match unsafe { std::ffi::CStr::from_ptr(s) }.to_str() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

pub struct DisplayHandle {
    native: ddcutil_sys::DDCA_Display_Handle,
}

impl DisplayHandle {
    pub fn non_table_vcp_value(
        &mut self,
        feature_code: ddcutil_sys::DDCA_Vcp_Feature_Code,
    ) -> Result<(u16, u16), Error> {
        let mut vcpval = VcpValue::default();
        let status = unsafe {
            ddcutil_sys::ddca_get_non_table_vcp_value(self.native, feature_code, &mut vcpval)
        };
        if status != DDCRC_OK {
            Err(Error::Sys(status))
        } else {
            let max_val = (vcpval.mh as u16) << 8 | vcpval.ml as u16;
            let cur_val = (vcpval.sh as u16) << 8 | vcpval.sl as u16;
            Ok((max_val, cur_val))
        }
    }

    pub fn set_non_table_vcp_value(
        &mut self,
        feature_code: ddcutil_sys::DDCA_Vcp_Feature_Code,
        value: u16,
    ) -> Result<(), Error> {
        let hi_byte = (value >> 8) as u8;
        let lo_byte = (value & 0xff) as u8;
        let status = unsafe {
            ddcutil_sys::ddca_set_non_table_vcp_value(self.native, feature_code, hi_byte, lo_byte)
        };
        if status != DDCRC_OK {
            Err(Error::Sys(status))
        } else {
            Ok(())
        }
    }
}

impl Drop for DisplayHandle {
    fn drop(&mut self) {
        let status = unsafe { ddcutil_sys::ddca_close_display(self.native) };
        if status != DDCRC_OK {
            panic!("ddca_close_display failed: {}", status);
        }
    }
}

impl std::fmt::Display for DisplayHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = unsafe { ddcutil_sys::ddca_dh_repr(self.native) };
        if s.is_null() {
            return Err(std::fmt::Error);
        }

        match unsafe { std::ffi::CStr::from_ptr(s) }.to_str() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}
