

use std::{io, fs, fmt};
use std::os::unix::io::{AsRawFd, RawFd};
use std::fs::File;
use std::ffi::OsStr;


//use nix;
//
//use usbtypes::*;
//use usbtypes::devfs::*;
//use deviceinfo::*;

use super::*;


/// Perform synchronous USB operations
///
/// This struct wraps a usbfs device for performing synchronous USB operations.  If all you need is
/// control transfer access to your USB hardware, this may be all you need.
pub struct Device(pub File);

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}


impl Device {
    /// Create new Device given a DeviceInfo struct.
    ///
    /// # Examples
    /// Find and open a specific device by idVendor and idProduct.
    ///
    /// ```
    /// use usbfs::*;
    ///
    /// fn main() {
    ///     let device_info = deviceinfo_collection()
    ///                       .into_iter()
    ///                       .find(is_my_device)
    ///                       .unwrap();
    ///     let device = Device::new(device_info);
    ///     // ...
    /// }
    ///
    /// fn is_my_device(di: &DeviceInfo) -> bool {
    ///     match di.device_descriptor() {
    ///         Ok(descr) => (descr.idVendor == 0xffff) && (descr.idProduct == 3),
    ///         _ => false
    ///     }
    /// }
    /// ```
    pub fn new(device: &DeviceInfo) -> io::Result<Self> {
        Self::from_busdev(device.busnum()?, device.devnum()?)
    }

    /// Open device from device path.  See [`DeviceInfo::from_devpath`]
    pub fn from_devpath<P: AsRef<OsStr>>(p: P) -> io::Result<Self> {
        Self::new(&DeviceInfo::from_devpath(p)?)
    }

    pub fn from_busdev(busnum: u32, devnum: u32) -> io::Result<Self> {
        let mut openopts = fs::OpenOptions::new();
        openopts.read(true).write(true);

        // pick first available path for device
        openopts.open(fmt::format(format_args!("/dev/bus/usb/{:03}/{:03}", busnum, devnum)))
            .or_else(|_|openopts.open(fmt::format(format_args!("/dev/usbdev{}.{}", busnum, devnum))))
            .or_else(|_|openopts.open(fmt::format(format_args!("/proc/bus/usb/{:03}/{:03}", busnum, devnum))))
        .map(|f| Device(f))
    }

    /// Perform a single synchronous control transfer.  Do not write a Setup packet to
    /// the `data` buffer; `data` is only for IN our OUT data and may be empty if
    /// no exchange beyond the Setup packet is needed.
    ///
    /// The number of bytes transferred to/from `data` is returned as the `Ok` result.
    pub fn control_transfer(&self,
                            setupdirection: SetupDirection,
                            setuptype: SetupType,
                            setuprecipient: SetupRecipient,
                            bRequest: u8,
                            wValue: u16,
                            wIndex: u16,
                            odata: Option<&mut [u8]>,
                            timeout_ms: u32)
                            -> io::Result<i32> {

        let (data, wLength) = match odata {
            Some(mref) => (mref.as_mut_ptr(), mref.len() as u16),
            None => (std::ptr::null_mut(), 0),
        };


        let mut xfer = devfs::CtrlTransfer {
            bmRequestType: (setupdirection as u8) | (setuptype as u8) | (setuprecipient as u8),
            bRequest,
            wValue,
            wIndex,
            wLength,
            timeout: timeout_ms,
            data,
        };

        unsafe { devfs::nix_result_to_io_result(devfs::control(self.as_raw_fd(), &mut xfer)) }
    }

    pub fn control_transfer_in(&self,
                            setuptype: SetupType,
                            setuprecipient: SetupRecipient,
                            bRequest: u8,
                            wValue: u16,
                            wIndex: u16,
                            odata: Option<&mut [u8]>,
                            timeout_ms: u32)
                            -> io::Result<i32> {

        let (data, wLength) = match odata {
            Some(mref) => (mref.as_mut_ptr(), mref.len() as u16),
            None => (std::ptr::null_mut(), 0),
        };

        let mut xfer = devfs::CtrlTransfer {
            bmRequestType: (SetupDirection::DeviceToHost as u8) | (setuptype as u8) | (setuprecipient as u8),
            bRequest,
            wValue,
            wIndex,
            wLength,
            timeout: timeout_ms,
            data,
        };

        unsafe { devfs::nix_result_to_io_result(devfs::control(self.as_raw_fd(), &mut xfer)) }
    }

    pub fn control_transfer_out(&self,
                            setuptype: SetupType,
                            setuprecipient: SetupRecipient,
                            bRequest: u8,
                            wValue: u16,
                            wIndex: u16,
                            odata: Option<& [u8]>,
                            timeout_ms: u32)
                            -> io::Result<i32> {

        let (data, wLength) = match odata {
            Some(mref) => (mref.as_ptr(), mref.len() as u16),
            None => (std::ptr::null(), 0),
        };


        let mut xfer = devfs::CtrlTransfer {
            bmRequestType: (SetupDirection::HostToDevice as u8) | (setuptype as u8) | (setuprecipient as u8),
            bRequest,
            wValue,
            wIndex,
            wLength,
            timeout: timeout_ms,
            data: data as *mut u8,
        };

        unsafe { devfs::nix_result_to_io_result(devfs::control(self.as_raw_fd(), &mut xfer)) }
    }



    pub fn claim_interface(&self, interface: u16) -> io::Result<()> {
        let i: devfs::c_uint = interface as devfs::c_uint;
        unsafe { devfs::nix_result_to_io_result(devfs::claiminterface(self.as_raw_fd(), &i).map(|_|())) }
    }

    pub fn set_interface(&self, interface: u32, altsetting: u32) -> io::Result<()> {
        unsafe {
            let data = devfs::SetInterface{
                interface: interface as devfs::c_uint,
                altsetting: altsetting as devfs::c_uint,
            };
            devfs::nix_result_to_io_result(devfs::setinterface(self.as_raw_fd(), &data)).map(|_|())
        }
    }
}
