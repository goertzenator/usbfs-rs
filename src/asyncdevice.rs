use super::*;
use std::io;
use std::ops::DerefMut;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;

#[cfg(feature = "mio")]
use mio::unix::EventedFd;
#[cfg(feature = "mio")]
use mio::{Evented, PollOpt, Token};

/// Low level URB-rendering trait for async transfers.
///
/// This trait is used by `AsyncDevice` to convert transfer objects into `Urb`s for submission
/// to the usbfs driver.  It is unsafe because undefined behavior can be invoked by improper `Urb`
/// setup.  Types implementing `Transfer` will typically contain an `Urb` struct and buffer, with the `Urb`
/// configured to read or write to the associated buffer.

pub unsafe trait Transfer {
    /// Prepare an URB for submission to usbfs driver.
    fn wire_urb(&mut self) -> &mut Urb;
}

// ///
// /// This type represents a single USB transfer.  It contains parameters
// /// for the transfer (an URB structure, USB Request Block) and a buffer
// /// to hold outgoing and/or incoming data.
// ///
// /// The type parameter `B` is the buffer type to be used for this transfer
// /// which must have `AsMut<[u8]>`.  `Vec<u8>` and `[u8;N]`
// /// meet the requirements and are excellent choices.  Other buffer types can used
// /// by implementing the required traits.
// ///
// /// (Note that [u8;N] has `AsMut` only for N up to 32.
// /// Future versions of Rust should remove this limit.)
// ///
// /// # Examples
// /// Implement a custom buffer type.  Large arrays might not have
// /// `AsMut`, but they still coerce to slices.

/// Perform asynchronous USB operations
///
/// Async USB operations are started by *submitting* a transfer object to an `AsyncDevice` and then
/// later *reaping* the transfer when it has completed.  `AsyncDevice` takes exclusive ownership of
/// transfer objects while they are being processed.
///
/// The transfer object has trait bound `DerefMut`, which in practice means `Box` or `&mut` (this
/// is what allows `AsyncDevice` to hold exclusive ownership).  The derefed type must also implement
/// `Transfer` so that an `Urb` can be acquired for the underlying usbfs driver.
///
/// `AsyncDevice` implements `AsRawFd` so that it can partake in external select/poll event loops.
/// The underlying file descriptor becomes *writable* when a transfer is ready to be reaped.

pub struct AsyncDevice<R>
//    where R: DerefMut,
//          R::Target: Transfer

// DerefMut isn't quite what we want because there is no garantee of stable references.
// Box and &mut do provide this, but it is coincidence.
// Possible future alternatives are Pin, Anchor, StableDeref.
{
    pub device: Device,
    transfers: Vec<Option<R>>,
}

impl<R> From<Device> for AsyncDevice<R>
//    where R: DerefMut,
//          R::Target: Transfer
{
    fn from(d: Device) -> Self {
        AsyncDevice {
            device: d,
            transfers: Default::default(),
        }
    }
}

impl<R> AsRawFd for AsyncDevice<R>
//    where R: DerefMut,
//          R::Target: Transfer
{
    fn as_raw_fd(&self) -> RawFd {
        self.device.as_raw_fd()
    }
}

#[allow(non_snake_case)]
impl<R> AsyncDevice<R>
where
    R: DerefMut,
    R::Target: Transfer,
{
    /// Create new AsyncDevice given a DeviceInfo struct.
    pub fn new(device: &DeviceInfo) -> io::Result<Self> {
        Device::new(device).map(|d| AsyncDevice {
            device: d,
            transfers: Default::default(),
        })
    }

    /// Submit a transfer for processing
    ///
    /// This method takes ownership of the provided transfer, invokes `wire_urb()`, and begins
    /// processing it asynchronously.  A result is returned immediately without
    /// waiting for completion.  The `Ok` result is a `slot` number that can later
    /// be used to `discard()` the transfer or identify it when `reap()`ed.  The `Err`
    /// result is a 2-tuple containing the error code and the original transfer.
    pub fn submit_give_back_on_fail(&mut self, mut transfer: R) -> Result<usize, (io::Error, R)> {
        let urbp: *mut Urb = transfer.wire_urb();

        let id = self.insert_transfer(transfer);
        unsafe {
            (*urbp).usercontext = id as usize;
        }

        match unsafe { devfs::nix_result_to_io_result(devfs::submiturb(self.as_raw_fd(), urbp)) } {
            Ok(_result) => {
                // keep transfer, return slot for later reference
                Ok(id)
            }
            Err(err) => {
                // return error, give transfer back
                Err((err, self.take_transfer(id).unwrap()))
            }
        }
    }

    /// Submit a transfer for processing
    ///
    /// Same as `submit_give_back_on_fail()`, but drop transfer upon failure.
    pub fn submit(&mut self, transfer: R) -> io::Result<usize> {
        self.submit_give_back_on_fail(transfer)
            .map_err(|(err, _)| err)
    }

    /// Collect a previously submitted transfer
    ///
    /// If no transfer has been completed the error kind will be `io::ErrorKind::WouldBlock`.
    /// The `Ok` result is a 3-tuple consisting of:
    /// * The `slot` of the reaped transfer.  This is the number that is returned by `submit()`.
    /// * The `Transfer` itself.
    /// * The result of the transfer.  The `Ok` value is the number of bytes transferred.
    ///
    /// # Examples
    /// Recipe for processing the return result:
    /// ```
    /// match device.reap_nowait() {
    ///     Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
    ///         // no transfers have finished
    ///         // ...
    ///     }
    ///     Ok((_slot, xfer, Ok(actual_length))) => {
    ///         // transfer completed!
    ///         let buf = xfer.result_buf(actual_length);
    ///         // ...
    ///     },
    ///     Ok((_slot, _xfer, Err(_err))) =>
    ///         // transfer returned failure!
    ///         // ...
    ///     Err(_err) =>
    ///         // underlying call to reap failed.  Device unplugged?
    ///         // ...
    /// }
    /// ```
    pub fn reap_nowait(&mut self) -> io::Result<R> {
        self.reap_main(false)
    }

    /// Wait for a previously submitted `Transfer` to finish.
    /// Similar to `read_nowait()`, but will wait for a transfer to complete before returning.
    /// Synchronous operation can be emulated by using `submit()`/`reap_wait()`
    ///
    /// # Examples
    /// The recipe for processing is slightly simpler than for `reap_nowait()`:
    /// ```
    /// match device.reap_wait() {
    ///     Ok((_slot, xfer, Ok(actual_length))) => {
    ///         // transfer completed!
    ///         let buf = xfer.result_buf(actual_length);
    ///         // ...
    ///     },
    ///     Ok((_slot, _xfer, Err(_err))) =>
    ///         // transfer returned failure!
    ///         // ...
    ///     Err(_err) =>
    ///         // underlying call to reap failed.  Device unplugged?
    ///         // ...
    /// }
    /// ```
    pub fn reap_wait(&mut self) -> io::Result<R> {
        self.reap_main(true)
    }

    // start abstracting transfer tracking so it can be traitified in the future

    fn insert_transfer(&mut self, transfer: R) -> usize {
        // find empty slot to stash this transfer
        let slot = {
            match self.transfers.iter().enumerate().find(|t| t.1.is_none()) {
                Some((i, _)) => {
                    self.transfers[i] = Some(transfer);
                    i
                }
                None => {
                    self.transfers.push(Some(transfer));
                    self.transfers.len() - 1
                }
            }
        };

        slot
    }

    fn take_transfer(&mut self, id: usize) -> Option<R> {
        self.transfers.get_mut(id).and_then(|e| e.take())
    }

    // fn get_transfer(&self, id: usize) -> Option<&R> {
    //     match self.transfers.get(id) {
    //         Some(&Some(ref t)) => Some(t),
    //         _ => None,
    //     }
    // }

    fn reap_main(&mut self, wait: bool) -> io::Result<R> {
        // get urb pointer
        let mut urbp: *mut Urb = ptr::null_mut();

        match wait {
            false => unsafe {
                devfs::nix_result_to_io_result(devfs::reapurbndelay(self.as_raw_fd(), &mut urbp))?
            },
            true => unsafe {
                devfs::nix_result_to_io_result(devfs::reapurb(self.as_raw_fd(), &mut urbp))?
            },
        };

        // get enclosing Transfer
        let id = unsafe { (*urbp).usercontext };
        Ok(self.take_transfer(id).unwrap())
    }

    // /// Abort an in-flight transfer by slot number.
    // /// The `Ok` result is the aborted transfer.  This operation will
    // /// fail if the transfer has already been queued for `reap()`ing.

    // FIXME: can't get address of URB from current Transfer impl or from get_transfer(). Something has to bend...

    pub fn discard(&mut self, _id: usize) -> io::Result<R> {
        panic!("not implemented");

        // match self.get_transfer(id) {
        //     Some(ref xfer) => {
        //         try!(unsafe{from_libc_result( libc::ioctl(self.as_raw_fd(),
        //             devfs::DISCARDURB_IOCTL,
        //             &xfer.urb as *const Urb))});
        //     },
        //     None =>
        //         return Err(io::Error::new(io::ErrorKind::Other, "invalid transfer id")),
        // };

        // Ok(self.take_transfer(id).unwrap())
    }
}

/// [mio](https://github.com/carllerche/mio) integration.
///
/// This trait allows `AsyncDevice` instances to partake in `mio` event loops.
/// `mio` integration (and dependency) can be turned off by disabling the
/// `mio` feature at the crate level.  This feature is enabled by default.
///
/// `Device`s become `Writeable` when `Transfer`s are available to be `reap()`ed.
#[cfg(feature = "mio")]
impl<R> Evented for AsyncDevice<R>
where
    R: DerefMut,
    R::Target: Transfer,
{
    fn register(
        &self,
        selector: &mut Selector,
        token: Token,
        interest: EventSet,
        opts: PollOpt,
    ) -> io::Result<()> {
        println!(
            "register {:?} {:?} {:?}",
            EventedFd(&self.as_raw_fd()),
            interest,
            opts
        );
        EventedFd(&self.as_raw_fd()).register(selector, token, interest, opts)
    }

    fn reregister(
        &self,
        selector: &mut Selector,
        token: Token,
        interest: EventSet,
        opts: PollOpt,
    ) -> io::Result<()> {
        println!("reregister");
        EventedFd(&self.as_raw_fd()).reregister(selector, token, interest, opts)
    }

    fn deregister(&self, selector: &mut Selector) -> io::Result<()> {
        println!("deregister");
        EventedFd(&self.as_raw_fd()).deregister(selector)
    }
}
