extern crate usbfs;
//extern crate mio;

use usbfs::*;
//use mio::*;
use std::thread::sleep;
use std::time::Duration;
use std::io;


/// This demo application performs a control transfer with a custom USB device using
/// the various techniques supported by the usbfs crate.

fn main() {
    my_test().unwrap();
}

#[derive(Default, Debug)]
#[repr(C)]
struct StreamFrame {
    samples: [[i32;8]; 2],     // samples from both inputs on all 8 ADCs.  samples[<subchannel>][<channel>]
    control_outputs: u32,      // gain and test signals in effect for following samples
//    gain_changed: u8,          // gain changed during below samples (1 bit per ADC, implies sample is invalid for that ADC)
    seq: u16,                  // frame sequence number.  Implies CONV phase.
    drdy_error: u8,            // indicates ADCs that failed to report DRDY
}

#[derive(Default, Debug)]
struct UrbDataFrame {
    streamframes: [[StreamFrame;9];10],  // 10 packets, up to 9 StreamFrames per packet
}

impl AsRef<[u8]> for UrbDataFrame {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.streamframes.as_ptr() as *const u8,
                self.streamframes.len() * std::mem::size_of_val(&self.streamframes[0]))
        }
    }
}

impl AsMut<[u8]> for UrbDataFrame {
    fn as_mut(&mut self) -> &mut [u8] {
//        println("{}, {}, {}", self.streamframes.len(), sl)
        unsafe {
            std::slice::from_raw_parts_mut(
                self.streamframes.as_mut_ptr() as *mut u8,
                self.streamframes.len() * std::mem::size_of_val(&self.streamframes[0]))
        }
    }
}

impl IsoBuffer for UrbDataFrame {
    fn packet_length(&self) -> usize {
        std::mem::size_of_val(&self.streamframes[0])
    }
//    fn packet_lengths(&self) -> impl Iterator<Item=usize> {
//        let packet_size = size_of_val(&self.streamframes[0]);
//        iter::repeat(packet_size) // infinite iterator, will be constrained by buffer size
//    }
}


/// Perform a synchronous transfer using USBFS blocking call.
fn my_test() -> io::Result<()> {
    let deviceinfo = get_my_device();
    let device = Device::new(&deviceinfo).unwrap();

    let mut unique_id:[u8;16] = [0x5a;16];

    device.control_transfer( SetupDirection::DeviceToHost,
                             SetupType::Vendor,
                             SetupRecipient::Interface,
                             3, // request (gets HW serial number)
                             0, // value (ignored for this request)
                             0, // index (Watchdog)
                             Some(&mut unique_id),
                             1000).unwrap();
    print!("HW serial = ");
    printbuf(&unique_id);

    device.claim_interface(2).unwrap();

    // select full DDC112 interface
    device.set_interface(2,1).unwrap();


    // start streaming
    device.control_transfer( SetupDirection::HostToDevice,
                             SetupType::Vendor,
                             SetupRecipient::Interface,
                             0, // request (STREAM_ON)
                             0, // value (ignored for this request)
                             2, // index (DDC112)
                             None,
                             1000).unwrap();


//
//    // stop streaming
//    device.control_transfer( SetupDirection::HostToDevice,
//                             SetupType::Vendor,
//                             SetupRecipient::Interface,
//                             1, // request (STREAM_OFF)
//                             0, // value (ignored for this request)
//                             2, // index (DDC112)
//                             None,
//                             1000).unwrap();


//    let asyncdevice = AsyncDevice::new(&deviceinfo).unwrap();
    let mut asyncdevice : AsyncDevice<_> = device.into();


    // get the transfers rolling
    for _ in 0..2 {
        let buf: UrbDataFrame = Default::default();
        let xfer: IsoBufTransfer<UrbDataFrame,32> = IsoBufTransfer::isochronous(
                                 0x81,  // endpoint
                                 UrbFlags::empty(),  // flags (none)
                                 buf );
//        println!("{:?}", xfer);
        asyncdevice.submit(Box::new(xfer)).unwrap();
    }



    //for _ in 0..10 {
    let mut count = 0;      // urb cycles
    let mut seq_faults: usize = 0;
    let mut seq = None;
//    let mut samples = 0;    // number of u32
//    let mut expected = 50;
    loop {
        count += 1;
        if (count%100) == 0 {
//            println!("samples = {:?}",samples);
        };

        match asyncdevice.reap_wait() {
            Ok(xfer) => {

                println!("urb seq = {}", count);
                println!("seq_faults = {}", seq_faults);
                for packet in xfer.status() {
                    println!("status={}, actual_length={}, length={}", packet.status, packet.actual_length, packet.length)
                }

                // iterate StreamFrames
                xfer.status().iter().zip(xfer.buf.streamframes.iter())
                    .flat_map(|(status, streamframes)| {
                        let cnt = match status.status {
                            0 => (status.actual_length as usize) / std::mem::size_of_val(&streamframes[0]),
                            _ => 0,
                        };
                        streamframes[..cnt].iter()
                    })
                    .for_each(|x| {
                        match (seq, x.seq) {
                            (None, _t) => (), // intialize seq
                            (Some(65535), 0) => (),
                            (Some(s), t) if (s+1)==t => (),
                            _ => {seq_faults +=1;},
                        }
                        seq = Some(x.seq);

//                        println!("{:?}",x)
                    });


                println!("streamframes[0][0] = {:?}", xfer.buf.streamframes[0][0]);
                println!("streamframes[0][1] = {:?}", xfer.buf.streamframes[0][1]);
                println!();

                asyncdevice.submit(xfer).unwrap();
            },
            // Ok((_slot, _xfer, Err(err))) => {
            //     //println!("error={:?}",err);
            //     panic!("transfer had an error!")
            // },
            Err(_err) =>
                panic!("reap had an error!"),
        };
    }; //loop
//    Ok(())
}


// fn make_transfer() -> StdTransfer<Vec<u8>> {
//     let mut xfer = StdTransfer::new(UrbType::Bulk, //  UrbType::Interrupt,
//                                  0x81,  // endpoint
//                                  UrbFlags::empty(),  // flags (none)
//                                  { let mut v=Vec::new();
//                                     v.resize(64*10,0);
//                                    v});

//     // make sure buffer is big enough for setup packet and result
//     //xfer.buf.resize(64*10,0);
//     xfer
// }

// fn make_control_transfer() -> StdTransfer<Vec<u8>> {
//     let mut xfer = StdTransfer::new(UrbType::Control,
//                                  0,  // endpoint (control)
//                                  UrbFlags::empty(),  // flags (none)
//                                  Vec::new());

//     // make sure buffer is big enough for setup packet and result
//     xfer.buf.resize(72,0);

//     // write setup packet to buffer
//     let setup = Setup::new(
//             SetupDirection::DeviceToHost,
//             SetupType::Vendor,
//             SetupRecipient::Interface,
//             0, // request for USB device, gets HW serial number
//             0, // value (ignored for this request)
//             0, // index (ignored for this request)
//             64,
//             );
//     write_setup_struct(&setup.to_bus(), &mut xfer.buf);

//     xfer
// }

// /// Perform a synchronous transfer using blocking reap.
// fn sync_transfer_demo() -> io::Result<()> {
//     println!("async_transfer_demo()");

//     let mut device = Device::new(&get_my_device()).unwrap();
//     device.submit(Box::new(make_transfer())).unwrap();
//     match device.reap_wait() {
//         Ok((_slot, xfer, Ok(actual_length))) => {
//             print!("HW serial = ");
//             printbuf(xfer.result_buf(actual_length));
//         },
//         Ok((_slot, _xfer, Err(_err))) =>
//             panic!("transfer had an error!"),
//         Err(_err) =>
//             panic!("reap had an error!"),
//     };
//     Ok(())
// }

// /// Perform an asynchronous transfer using polling and nonblocking reap.
// fn async_transfer_poll_demo() -> io::Result<()> {
//     println!("async_transfer_poll_demo()");

//     let mut device = Device::new(&get_my_device()).unwrap();
//     device.submit(Box::new(make_transfer())).unwrap();
//     loop {
//         match device.reap_nowait() {
//             Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
//                 sleep(Duration::from_millis(1));
//                 println!("Zzz..");
//                 continue;
//             }
//             Ok((_slot, xfer, Ok(actual_length))) => {
//                 print!("HW serial = ");
//                 printbuf(xfer.result_buf(actual_length));
//                 break;
//             },
//             Ok((_slot, _xfer, Err(_err))) =>
//                 panic!("transfer had an error!"),
//             Err(_err) =>
//                 panic!("reap had an error!"),
//         }
//     }
//     Ok(())
// }

// /// Perform asynchronous transfers using nonblocking reap and mio.
// fn async_transfer_mio_demo() -> io::Result<()> {
//     println!("async_transfer_mio_demo()");

//     // setup mio handler and event loop
//     let mut myhandler = MyHandler{device: Device::new(&get_my_device()).unwrap()};
//     let mut event_loop = EventLoop::new().unwrap();
//     event_loop.register(&myhandler.device, Token(0), EventSet::writable(),
//                     PollOpt::level()).unwrap();

//     // start the first transfer
//     myhandler.device.submit(Box::new(make_transfer())).unwrap();

//     // run the loop forever
//     //event_loop.run(&mut myhandler).unwrap();

//     // run the loop a few times
//     for _ in 0..10 {
//         event_loop.run_once(&mut myhandler, None).unwrap();
//     }
//     Ok(())
// }

// struct MyHandler {
//     device: Device<Vec<u8>>,
// }

// impl Handler for MyHandler {
//     type Timeout = ();
//     type Message = ();

//     fn ready(&mut self, _event_loop: &mut EventLoop<MyHandler>, _token: Token, _: EventSet) {
//         let r = self.device.reap_nowait();

//         match r {
//             Ok((_slot, xfer, Ok(actual_length))) => {
//                 print!("HW serial = ");
//                 printbuf(xfer.result_buf(actual_length));

//                 // start another transfer
//                 self.device.submit(xfer).unwrap();
//             },
//             _ => (),
//         }
//     }
// }


// /// Demonstrate transfer abort.
// fn discard_demo() -> io::Result<()> {
//     println!("discard_demo()");

//     let mut device = Device::new(&get_my_device()).unwrap();

//     device.submit(Box::new(make_transfer())).unwrap();
//     let slot = device.submit(Box::new(make_transfer())).unwrap();

//     device.discard(slot).unwrap();
//     println!("poof!");

//     Ok(())
// }


fn get_my_device() -> DeviceInfo {
    deviceinfo_enumerate().find(is_my_device).unwrap()
}

// find my custom LPCxpresso device
fn is_my_device(di: &DeviceInfo) -> bool {
    match di.device_descriptor() {
        Ok(descr) => (descr.idVendor == 0xffff) && (descr.idProduct == 4),
        _ => false
    }
}


fn printbuf(buf: &[u8]) {
    for &byte in buf.iter() {
        print!("{:02x}", byte);
    }
    println!();
}
