//! Usbfs is a linux device driver that gives userland applications access to USB devices.
//! This crate provides a Rust interface to usbfs and USB-related parts of sysfs.
//!
//! # Features
//! * Access to synchronous and asynchronous usbfs functions.
//! * Enumeration of USB devices using sysfs.
//! * The `usbfs::Device` type has the `mio::Evented` trait and can partake in [`mio`](https://github.com/carllerche/mio) event loops.
//! * Is written entirely in Rust.  The only external requirement is support for usbfs in the kernel.
//!
//! # Differences from `libusb`
//! [`libusb`](http://www.libusb.org/) is a multiplatform C library that provides userland access to USB.
//! Since many readers may be familiar with libusb, here is a list of differences between `libusb` and `usbfs`:
//!
//! * This crate is for Linux only.  `Libusb` is portable to many platforms including Linux, Windows, and OSX.
//! * This crate requires no initialization, no `context` structure, and has less coupling.
//! * This crate externalizes event loop support.  The `AsyncDevice` has traits
//!   [`AsRawFd`](https://doc.rust-lang.org/std/os/unix/io/trait.AsRawFd.html) and
//!   [`mio::Evented`](https://github.com/carllerche/mio) to facilitate integration into external
//!   event loops.
//!
//! Note that `libusb` has a [Rust wrapper](https://github.com/dcuddeback/libusb-rs).
//!
//! # Examples
//!
//! A basic synchronous transfer:
//!
//! ```
//! use usbfs::*;
//! fn main() {
//!     // find my device
//!     let device_info = deviceinfo_collection().into_iter().find(is_my_device).unwrap();
//!
//!     // open my device
//!     let device = Device::new(&device_info).unwrap();
//!
//!     // read some data from my device with a control transfer
//!     let mut unique_id:[u8;16] = unsafe{std::mem::uninitialized()};
//!     device.control_transfer( SetupDirection::DeviceToHost,
//!                              SetupType::Vendor,
//!                              SetupRecipient::Interface,
//!                              0, // request for USB device, gets HW serial number
//!                              0, // value (ignored for this request)
//!                              0, // index (ignored for this request)
//!                              &mut unique_id,
//!                              1000).unwrap();
//!
//!     // do stuff with unique_id ...
//! }
//!
//!
//! fn is_my_device(di: &DeviceInfo) -> bool {
//!     match di.device_descriptor() {
//!         Ok(descr) => (descr.idVendor == 0xffff) && (descr.idProduct == 3),
//!         _ => false
//!     }
//! }
//! ```


#![allow(non_snake_case)]

extern crate libc;

#[macro_use]
extern crate nix;

#[macro_use]
extern crate bitflags;

#[cfg(feature="mio")]
extern crate mio;

mod usbtypes;
pub use usbtypes::*;

mod devfs;
pub use devfs::{UrbType, UrbFlags};
//pub use devfs::UrbFlags; //::{URB_SHORT_NOT_OK, URB_ISO_ASAP, URB_BULK_CONTINUATION, URB_NO_FSBR,
                //URB_ZERO_PACKET, URB_NO_INTERRUPT};
pub use devfs::{Urb, IsoPacketDesc};

mod deviceinfo;
pub use deviceinfo::*;

mod device;
pub use device::*;

mod asyncdevice;
pub use asyncdevice::*;

mod monotransfer;
pub use monotransfer::*;

mod stdbuftransfer;
pub use stdbuftransfer::*;

mod isobuftransfer;
pub use isobuftransfer::*;
