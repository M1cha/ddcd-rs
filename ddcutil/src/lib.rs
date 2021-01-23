use std::convert::TryInto;

use ddcutil_sys::DDCA_Non_Table_Vcp_Value as VcpValue;
pub const DDCRC_OK: ddcutil_sys::DDCA_Status = ddcutil_sys::DDCRC_OK as ddcutil_sys::DDCA_Status;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("DDCA_Status:{0}")]
    Sys(ddcutil_sys::DDCA_Status),
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),
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
            Ok(DisplayRef {
                native: dref,
                owned: true,
            })
        }
    }
}

pub struct DisplayRef {
    native: ddcutil_sys::DDCA_Display_Ref,
    owned: bool,
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
        if !self.owned {
            return;
        }

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

pub struct DisplayInfoList {
    native: *mut ddcutil_sys::DDCA_Display_Info_List,
}

impl Drop for DisplayInfoList {
    fn drop(&mut self) {
        unsafe { ddcutil_sys::ddca_free_display_info_list(self.native) };
    }
}

impl DisplayInfoList {
    pub fn new(include_invalid_displays: bool) -> Result<Self, Error> {
        let mut dil = std::ptr::null_mut();
        let status =
            unsafe { ddcutil_sys::ddca_get_display_info_list2(include_invalid_displays, &mut dil) };
        if status != DDCRC_OK {
            Err(Error::Sys(status))
        } else {
            assert!(!dil.is_null());
            Ok(Self { native: dil })
        }
    }

    pub fn iter(&self) -> DislayInfoListIter {
        DislayInfoListIter { list: self, pos: 0 }
    }

    pub fn len(&self) -> usize {
        let native = unsafe { self.native.as_ref() }.unwrap();
        native.ct.try_into().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        let native = unsafe { self.native.as_ref() }.unwrap();
        native.ct == 0
    }
}

pub struct DislayInfoListIter<'a> {
    list: &'a DisplayInfoList,
    pos: usize,
}

impl<'a> Iterator for DislayInfoListIter<'a> {
    type Item = DisplayInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let native = unsafe { self.list.native.as_ref() }.unwrap();

        if self.pos < self.list.len() {
            let di = DisplayInfo {
                native: unsafe { native.info.as_ptr().add(self.pos) },
                pd: std::marker::PhantomData,
            };

            self.pos += 1;

            Some(di)
        } else {
            None
        }
    }
}

pub struct DisplayInfo<'a> {
    native: *const ddcutil_sys::DDCA_Display_Info,
    pd: std::marker::PhantomData<&'a DisplayInfoList>,
}

impl<'a> DisplayInfo<'a> {
    pub fn display_ref(&self) -> DisplayRef {
        let native = unsafe { self.native.as_ref() }.unwrap();
        DisplayRef {
            native: native.dref,
            owned: false,
        }
    }

    pub fn dispno(&self) -> usize {
        let native = unsafe { self.native.as_ref() }.unwrap();
        native.dispno.try_into().unwrap()
    }

    pub fn model(&self) -> &str {
        let native = unsafe { self.native.as_ref() }.unwrap();
        unsafe {
            std::ffi::CStr::from_ptr(native.model_name.as_ptr())
                .to_str()
                .unwrap()
        }
    }
}
