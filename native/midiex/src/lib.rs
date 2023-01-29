#![allow(unused)]
#![feature(drain_filter)]
extern crate midir;

// CORE MIDI TEST
use coremidi::{Destinations, Endpoint, Sources};

use std::sync::Mutex;
use std::sync::RwLock; // Potentially use this instead of Mutex for MidiInput and MidiOutput
use std::result::Result;

// PLAY TEST
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::io::{stdin, stdout, Write};


// Send messages to Erlang PID
// use tokio::sync::mpsc::{channel, Receiver, Sender};

use midir::{MidiInput, MidiOutput, MidiInputConnection, MidiOutputConnection, MidiInputPort, MidiOutputPort, Ignore, InitError};

use rustler::{Atom, Env, Error, NifResult, NifStruct, NifMap, NifTuple, ResourceArc, Term};



// This version uses threadlocal to create the MidiInput and MidiOutput objects
thread_local!(static GLOBAL_MIDI_INPUT_RESULT: Result<MidiInput, InitError> = MidiInput::new("MIDIex"));
thread_local!(static GLOBAL_MIDI_OUTPUT_RESULT: Result<MidiOutput, InitError> = MidiOutput::new("MIDIex"));



// Atoms
mod atoms {
    rustler::atoms! {
        ok,
        error,

        input,
        output,

        message
    }
}




// Send message to Erlang
#[rustler::nif]
pub fn subscribe(env: Env) -> Atom {  

    let pid = env.pid();

    let mut the_message = Term::map_new(env);

    the_message = the_message.map_put("message", "You recieved this message from Rust.").unwrap();


    env.send(&pid.clone(), the_message);

    sleep(Duration::from_millis(1000));
 
        

    atoms::ok()
}


// fn poll(env: Env, resource: ResourceArc<Ref>) -> (Atom, ResourceArc<Ref>) {
//     send(resource.clone(), Msg::Poll(env.pid()));

//     (ok(), resource)
// }




// fn deliver(env: Env, resource: ResourceArc<Ref>, msg: Message) -> (Atom, ResourceArc<Ref>) {
//     send(resource.clone(), Msg::Send(env.pid(), msg));
//     (ok(), resource)
// }





// CORE MIDI
#[rustler::nif(schedule = "DirtyCpu")]
fn try_core_midi() -> Result<(), Error> {
    println!("System destinations:");

    for (i, destination) in Destinations.into_iter().enumerate() {
        let display_name = get_display_name(&destination);
        println!("System sources:");
    }

    println!();
    println!("System sources:");

    for (i, source) in Sources.into_iter().enumerate() {
        let display_name = get_display_name(&source);
        println!("[{}] {}", i, display_name);
    }

    Ok(())
}

fn get_display_name(endpoint: &Endpoint) -> String {
    endpoint
        .display_name()
        .unwrap_or_else(|| "[Unknown Display Name]".to_string())
}




#[rustler::nif]
fn connect(midi_port: MidiPort) -> Result<OutConn, Error>{

    if midi_port.direction == atoms::output()  {
        println!("OUTPUT");

        let mut midi_output = MidiOutput::new("MIDIex").expect("Midi output");  

        // let mut port_ref = midi_port.port_ref.0;

        if let MidiexMidiSimplePortRef::Output(port) = &midi_port.port_ref.0 { 

            println!("OUTPUT PORT");

            let mut conn_out_result = midi_output.connect(&port, "MIDIex");

            let mut conn_out = match conn_out_result {
                Ok(conn_out) => {
                    println!("CONNECTION MADE");
                    
                    return Ok(
                        OutConn {
                            conn_ref: ResourceArc::new(OutConnRef::new(conn_out)),
                            midi_port: midi_port,          
                        }
                    )

                },
                Err(error) => panic!("Midi Output Connection Error: Problem getting midi output connection. Error: {:?}", error)
            };
    
            

         };


    } else {
        println!("INPUT");

        let mut midi_input = MidiInput::new("MIDIex").expect("Midi output");  

        // let mut port_ref = midi_port.port_ref.0;

        if let MidiexMidiSimplePortRef::Input(port) = &midi_port.port_ref.0 { 

            println!("INPUT PORT");

            // let mut conn_in_result = midi_input.connect(&port, "MIDIex");

            // let mut conn_in = match conn_in_result {
            //     Ok(conn_in) => { println!("CONNECTION MADE"); conn_in},
            //     Err(error) => panic!("Midi Input Connection Error: Problem getting midi input connection. Error: {:?}", error)
            // };

            return Err(Error::RaiseTerm(Box::new(
                "Input connection rather than output.".to_string(),
            )))
    
            

         };
    }



    Err(Error::RaiseTerm(Box::new(
        "No output connection".to_string(),
    )))
}


#[rustler::nif(schedule = "DirtyCpu")]
fn play() -> Result<(), Error>{
 
    // let midi_output = midi_io.resource_output.0.lock().unwrap();

    let midi_output_result: Result<MidiOutput, InitError> = MidiOutput::new("MIDIex");
    

    let midi_output = match midi_output_result {
        Ok(midi_device) => midi_device,
        Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
    };


    let out_ports = midi_output.ports();
    let out_port: &MidiOutputPort = &out_ports[1];

    let mut conn_out_result = midi_output.connect(&out_port, "MIDIex");

    let mut conn_out = match conn_out_result {
        Ok(conn_out) => conn_out,
        Err(error) => panic!("Midi Output Connection Error: Problem getting midi output connection. Error: {:?}", error)
    };

    println!("Connection open. Listen!");
    {
        // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
        let mut play_note = |note: u8, duration: u64| {
            const NOTE_ON_MSG: u8 = 0x90;
            const NOTE_OFF_MSG: u8 = 0x80;
            const VELOCITY: u8 = 0x64;
            // We're ignoring errors in here
            let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
            sleep(Duration::from_millis(duration * 150));
            let _ = conn_out.send(&[NOTE_OFF_MSG, note, VELOCITY]);
        };

        sleep(Duration::from_millis(4 * 150));
        
        play_note(66, 4);
        play_note(65, 3);
        play_note(63, 1);
        play_note(61, 6);
        play_note(59, 2);
        play_note(58, 4);
        play_note(56, 4);
        play_note(54, 4);
    }
    sleep(Duration::from_millis(150));
    println!("\nClosing connection");
    // This is optional, the connection would automatically be closed as soon as it goes out of scope
    conn_out.close();
    println!("Connection closed");
  
    Ok(())
}



pub fn get_first_midi_out_device(midi_out: &mut MidiOutput) -> Result<MidiOutputPort, Error> {
    let out_ports = midi_out.ports();
    if out_ports.len() == 0 {
        panic!("No MIDI devices attached")
    } else {
        let device_name = midi_out.port_name(&out_ports[0]).expect("Device name not available");
        println!("Chose MIDI device {device_name}");
        Ok(out_ports[0].clone())
    }
}


#[rustler::nif(schedule = "DirtyCpu")]
fn play_two(midi_out_conn: OutConn) -> Result<(), Error>{
 

    let mut midi_output = MidiOutput::new("MIDIex").expect("Midi output");


    // let out_port = get_first_midi_out_device(&mut midi_output).expect("Midi output port");
    let mut conn_out = midi_out_conn.conn_ref.0.lock().unwrap();


    // let mut conn_out_result = midi_output.connect(&out_port, "MIDIex");

    // let mut conn_out = match conn_out_result {
    //     Ok(conn_out) => conn_out,
    //     Err(error) => panic!("Midi Output Connection Error: Problem getting midi output connection. Error: {:?}", error)
    // };

    

    println!("Connection open. Listen!");
    {
        // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
        let mut play_note = |note: u8, duration: u64| {
            const NOTE_ON_MSG: u8 = 0x90;
            const NOTE_OFF_MSG: u8 = 0x80;
            const VELOCITY: u8 = 0x64;
            // We're ignoring errors in here
            let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
            sleep(Duration::from_millis(duration * 150));
            let _ = conn_out.send(&[NOTE_OFF_MSG, note, VELOCITY]);
        };

        sleep(Duration::from_millis(4 * 150));
        
        play_note(66, 4);
        play_note(65, 3);
        play_note(63, 1);
        play_note(61, 6);
        play_note(59, 2);
        play_note(58, 4);
        play_note(56, 4);
        play_note(54, 4);
    }
    sleep(Duration::from_millis(150));
    println!("\nClosing connection");
    // This is optional, the connection would automatically be closed as soon as it goes out of scope
    // conn_out.close();
    println!("Connection closed");





    // thread::spawn(move || {
    //     // let mut midi = Midi {
    //     //     sender: ch_in,
    //     //     input: MidiInput::new("ports").expect("Midi input should work"),
    //     //     active_connections: Vec::new(),
    //     // };
    //     // loop {
    //     //     midi.tick();
    //     //     thread::sleep(Duration::from_millis(1000));
    //     // }
    //     // let mut midi_output = midi_io.resource_output.0.lock().unwrap();
    //     // let mut midi_output = &mut *midi_output;

    //     // let mut s = s.lock().expect("mutex error");
       

    //     // let lock = midi_io.resource_output.0.lock().expect("Failed to obtain a lock");

    //     // let mut midi_output = lock.clone();


    //     let midi_output_result: Result<MidiOutput, InitError> = MidiOutput::new("MIDIex");

    //     let mut midi_output = match midi_output_result {
    //         Ok(midi_device) => midi_device,
    //         Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
    //     };
    
    
    //     let out_port = get_first_midi_out_device(&mut midi_output).expect("Midi output port");

    //     // let out_ports = midi_output.ports();
    //     // let out_port: &MidiOutputPort = &out_ports[1];

    //     // let mut conn_out_result = midi_output.connect(&out_port, "MIDIex");


    //     let mut conn_out_result = midi_output.connect(&out_port, "MIDIex");

    //     let mut conn_out = match conn_out_result {
    //         Ok(conn_out) => conn_out,
    //         Err(error) => panic!("Midi Output Connection Error: Problem getting midi output connection. Error: {:?}", error)
    //     };

    //     println!("Connection open. Listen!");
    //     {
    //         // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
    //         let mut play_note = |note: u8, duration: u64| {
    //             const NOTE_ON_MSG: u8 = 0x90;
    //             const NOTE_OFF_MSG: u8 = 0x80;
    //             const VELOCITY: u8 = 0x64;
    //             // We're ignoring errors in here
    //             let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
    //             sleep(Duration::from_millis(duration * 150));
    //             let _ = conn_out.send(&[NOTE_OFF_MSG, note, VELOCITY]);
    //         };

    //         sleep(Duration::from_millis(4 * 150));
            
    //         play_note(66, 4);
    //         play_note(65, 3);
    //         play_note(63, 1);
    //         play_note(61, 6);
    //         play_note(59, 2);
    //         play_note(58, 4);
    //         play_note(56, 4);
    //         play_note(54, 4);
    //     }
    //     sleep(Duration::from_millis(150));
    //     println!("\nClosing connection");
    //     // This is optional, the connection would automatically be closed as soon as it goes out of scope
    //     conn_out.close();
    //     println!("Connection closed");


    // });


    
  
    Ok(())
}





// MIDI Connection

// pub struct MidexMidiInputConnectionRef(pub Mutex<MidiInputConnection>);
// pub struct MidexMidiOutputConnectionRef(pub Mutex<MidiOutputConnection>);

// impl MidexMidiInputConnectionRef {
//     pub fn new(data: MidiInputConnection) -> Self {
//         Self(Mutex::new(data))
//     }
// }

// impl MidexMidiOutputConnectionRef {
//     pub fn new(data: MidiOutputConnection) -> Self {
//         Self(Mutex::new(data))
//     }
// }




// pub enum MidiexConnRef {
//     Input(MidiInputConnection), 
//     Output(MidiOutputConnection), 
//   }


// pub struct FlexiConn(pub MidiexConnRef);

// impl FlexiConn {
//     pub fn new(data: MidiexConnRef) -> Self {
//         Self(data)
//     }
// }
  

#[derive(NifStruct)]
#[module = "Midiex.OutConn"]
pub struct OutConn {
    conn_ref: ResourceArc<OutConnRef>,
    midi_port: MidiPort
    // port_name: String,
    // port_num: usize,
    // port_ref: ResourceArc<FlexiPort>
}

pub struct OutConnRef(pub Mutex<MidiOutputConnection>);

impl OutConnRef {
    pub fn new(data: MidiOutputConnection) -> Self {
        Self(Mutex::new(data))
    }
}




// MIDI Ports


// pub struct MidiexMidiInputPortRef(pub MidiInputPort);

// impl MidiexMidiInputPortRef {
//     pub fn new(data: MidiInputPort) -> Self {
//         // Self(Mutex::new(data))
//         Self(data)
//     }
// }

// pub struct MidiexMidiOutputPortRef(pub MidiOutputPort);

// impl MidiexMidiOutputPortRef {
//     pub fn new(data: MidiOutputPort) -> Self {
//         //  Self(Mutex::new(data))
//          Self(data)

//     }
// }

// pub enum MidiexMidiPortRef {
//     Input(MidiexMidiInputPortRef), 
//     Output(MidiexMidiOutputPortRef), 
//   }


pub enum MidiexMidiSimplePortRef {
    Input(MidiInputPort), 
    Output(MidiOutputPort), 
  }


pub struct FlexiPort(pub MidiexMidiSimplePortRef);

impl FlexiPort {
    pub fn new(data: MidiexMidiSimplePortRef) -> Self {
        Self(data)
    }
}
  


#[derive(NifStruct)]
#[module = "Midiex.MidiPort"]
pub struct MidiPort {
    direction: Atom,
    name: String,
    num: usize,
    // port_ref: ResourceArc<MidiexMidiPortRef>,
    port_ref: ResourceArc<FlexiPort>,
}




#[derive(NifMap)]
pub struct NumPorts {
    input: usize,
    output: usize 
}




// MIDI IO related 

pub struct MidiexMidiInputRef(pub Mutex<MidiInput>);
pub struct MidiexMidiOutputRef(pub Mutex<MidiOutput>);

impl MidiexMidiInputRef {
    pub fn new(data: MidiInput) -> Self {
        Self(Mutex::new(data))
    }
}

impl MidiexMidiOutputRef {
    pub fn new(data: MidiOutput) -> Self {
        Self(Mutex::new(data))
    }
}

// pub struct MidiexMidiInputRef(pub RwLock<MidiInput>);
// pub struct MidiexMidiOutputRef(pub RwLock<MidiOutput>);

// impl MidiexMidiInputRef {
//     pub fn new(data: MidiInput) -> Self {
//         Self(RwLock::new(data))
//     }
// }

// impl MidiexMidiOutputRef {
//     pub fn new(data: MidiOutput) -> Self {
//         Self(RwLock::new(data))
//     }
// }

#[derive(NifStruct)]
#[module = "Midiex.MidiIO"]
pub struct MidiexMidiIO {
    pub resource_input: ResourceArc<MidiexMidiInputRef>,
    pub resource_output: ResourceArc<MidiexMidiOutputRef>,
    pub active_connections: Vec<MidiPort>,
}

impl MidiexMidiIO {
    pub fn new(midi_input: MidiInput, midi_output: MidiOutput) -> Self {
        Self {
            resource_input: ResourceArc::new(MidiexMidiInputRef::new(midi_input)),
            resource_output: ResourceArc::new(MidiexMidiOutputRef::new(midi_output)),
            active_connections: Vec::new()
        }
    }

    // pub fn connect_first_port(&mut self) -> MidiOutputConnection {
         
    //     let mut midi_output = self.resource_output.0.lock().expect("mutex error");
    //     let out_port = get_first_midi_out_device(&mut midi_output).expect("Midi output port");

    //     midi_output.connect(&out_port, "MIDIex").expect("Midi output connection to port")

    // }

    // pub fn refresh_connections(&mut self) -> Self {



    // }

    // fn refresh_connections(&mut self) {

    //     let mut midi_input = self.resource_input.0.lock().unwrap();
    //     midi_input.ignore(Ignore::None);

    //     // Drop connections that are no longer active.
    //     self.active_connections
    //         .drain_filter(|connection| midi_input.port_name(&connection.input_port).is_err())
    //         .for_each(|connection| {
    //             println!("`{}` was disconnected", connection.name);
    //         });

    //     // If active connections count is equal to count of available ports
    //     // we shall consider it as if we are not in need to open new ones.
    //     if self.active_connections.len() == self.midi_input.port_count() {
    //         return;
    //     }

    //     for (i, port) in self.midi_input.ports() {
    //         // First grab name of the connection
    //         //
    //         // Its a requirement to check if this port is valid, because
    //         // CoreMIDI may hold some kind of phantom connection
    //         // even after all devices will be disconnected.
    //         let name = match self.midi_input.port_name(&port) {
    //             Ok(name) => name,
    //             Err(_) => {
    //                 println!("Failed to retrieve a port name for a new port");
    //                 continue;
    //             }
    //         };

    //         // midir does not allow to compare unique id of ports so i use it's name
    //         let already_connected = self.active_connections.iter().any(|x| x.name == name);

    //         if !already_connected {
    //             let input = MidiInput::new("MIDIex").expect("Midi input should work");
    //             // let conn_in = input
    //             //     .connect(&port, "midir-read-input", listener, self.sender.clone())
    //             //     .unwrap();
    //             println!("Opened input connection to '{}'", name);
    //             // self.active_connections.push(Connection {
    //             //     input_port: port,
    //             //     _keep_alive: conn_in,
    //             //     name,
    //             // });

    //             self.active_connections.push(
    //                 MidiPort{
    //                     direction: atoms::input(),
    //                     name: name,
    //                     num: i,
    //                     input_port: port
    //             });
            

    //         }
    //     }
    // }

}




#[rustler::nif(schedule = "DirtyCpu")]
fn try_connect(midi_io: MidiexMidiIO) -> Result<(), Error> {

    let mut vec_of_output_ports: Vec<&MidiOutputPort> = Vec::new();
    let mut vec_of_input_ports: Vec<&MidiInputPort> = Vec::new();

    let mut midi_input = midi_io.resource_input.0.lock().unwrap();
    midi_input.ignore(Ignore::None);
    let midi_output = midi_io.resource_output.0.lock().unwrap();

    let in_ports = midi_input.ports();
    let out_ports = midi_output.ports();
    let num_ports = out_ports.len();

    println!("\n{:?} MIDI output ports:\r", num_ports);

    for (i, p) in midi_output.ports().iter().enumerate() {

        let port_name = if let Ok(port_name) = midi_output.port_name(&p) { port_name } else { "No device name given".to_string() };

        println!("\t{:?}\t {:?}\r", i, port_name);

        let out_port: &MidiOutputPort = out_ports.get(i).unwrap();

        // midi_io.active_connections
        //     .drain_filter(|connection| self.input.port_name(&connection.input_port).is_err())
        //     .for_each(|connection| {
        //         println!("`{}` was disconnected", connection.name);
        //     });   


        println!("\t{} with name {:?} has an error: {:?}\r", i, midi_output.port_name(out_port), midi_output.port_name(out_port).is_err());
        println!("\t{} with name {:?} has an error: {:?}\r\n", i, midi_output.port_name(p), midi_output.port_name(p).is_err());


        // let mut new_conn = MidiOutput::connect(midi_output, p, "");
        // let  new_conn = midi_output.connect(out_port, "");

        // println!("\t{} Conn {:?}\r", i, new_conn);

        vec_of_output_ports.push(out_port);

        


    }
    // println!("\n{:?} MIDI output ports.\r", vec_of_output_ports.len());




    println!("\n\r{:?} MIDI input ports:\r", num_ports);

    for (i, p) in midi_input.ports().iter().enumerate() {

        let port_name = if let Ok(port_name) = midi_input.port_name(&p) { port_name } else { "No device name given".to_string() };

        println!("\t{:?}\t {:?}\r", i, port_name);

        let in_port: &MidiInputPort = in_ports.get(i).unwrap();

        // midi_io.active_connections
        //     .drain_filter(|connection| self.input.port_name(&connection.input_port).is_err())
        //     .for_each(|connection| {
        //         println!("`{}` was disconnected", connection.name);
        //     });   


        println!("\t{} with name {:?} has an error: {:?}\r", i, midi_input.port_name(in_port), midi_input.port_name(in_port).is_err());
        println!("\t{} with name {:?} has an error: {:?}\r\n", i, midi_input.port_name(p), midi_input.port_name(p).is_err());

        vec_of_input_ports.push(in_port);


    }
    // println!("\n{:?} MIDI input ports.\r", vec_of_input_ports.len());


    Ok(())
}


// List all the ports, taking midi_io as input
#[rustler::nif(schedule = "DirtyCpu")]
fn list_ports() -> Result<Vec<MidiPort>, Error> {

    
    // let midi_output = midi_io.resource_output.0.lock().unwrap();
    // let mut midi_input = midi_io.resource_input.0.lock().unwrap();
    // midi_input.ignore(Ignore::None);

    let mut vec_of_devices: Vec<MidiPort> = Vec::new();

    GLOBAL_MIDI_INPUT_RESULT.with(|midi_input_result| {

        let midi_input = match midi_input_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi input devices. Error: {:?}", error)
        };

        println!("\nMidi input ports: {:?}\n\r", midi_input.port_count());

        for (i, p) in midi_input.ports().iter().enumerate() {
        
            let port_name = if let Ok(port_name) = midi_input.port_name(&p) { port_name } else { "No device name given".to_string() };
    
                vec_of_devices.push(
                    MidiPort{
                        direction: atoms::input(),
                        name: port_name,
                        num: i,
                        port_ref:
                        ResourceArc::new(FlexiPort::new(MidiexMidiSimplePortRef::Input(MidiInputPort::clone(p))))
            
                    });
        
        }
    
    });

    GLOBAL_MIDI_OUTPUT_RESULT.with(|midi_output_result| {

        let midi_output = match midi_output_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
        };

        println!("Midi output ports: {:?}\n\r", midi_output.port_count());

        for (i, p) in midi_output.ports().iter().enumerate() {  
        
            let port_name = if let Ok(port_name) = midi_output.port_name(&p) { port_name } else { "No device name given".to_string() };
          
                vec_of_devices.push(
                    MidiPort{
                        direction: atoms::output(),
                        name: port_name,
                        num: i,
                        port_ref: ResourceArc::new(
                                FlexiPort::new(MidiexMidiSimplePortRef::Output(MidiOutputPort::clone(p)))
                            )
                    });
    
        }

    });


    return Ok(vec_of_devices)

}


#[rustler::nif(schedule = "DirtyCpu")]
fn count_ports() -> Result<NumPorts, Error>{

    let mut num_input_ports = 0;
    let mut num_output_ports = 0;

    GLOBAL_MIDI_INPUT_RESULT.with(|midi_input_result| {

        let midi_input = match midi_input_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi input devices. Error: {:?}", error)
        };

        num_input_ports = midi_input.port_count();

    });

    GLOBAL_MIDI_OUTPUT_RESULT.with(|midi_output_result| {

        let midi_output = match midi_output_result {
            Ok(midi_device) => midi_device,
            Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
        };

        num_output_ports = midi_output.port_count();

    });

    return Ok( NumPorts { input: num_input_ports, output:  num_output_ports } )
}






   
// #[rustler::nif(schedule = "DirtyCpu")]
// fn devices() -> Result<Vec<MidiDevice>, Error>{

//     let mut vec_of_devices: Vec<MidiDevice> = Vec::new();

//     GLOBAL_MIDI_INPUT_RESULT.with(|midi_input_result| {

//         let midi_input = match midi_input_result {
//             Ok(midi_device) => midi_device,
//             Err(error) => panic!("Problem getting midi input devices. Error: {:?}", error)
//         };

//         println!("\nMidi input ports: {:?}\n\r", midi_input.port_count());

//         for (i, p) in midi_input.ports().iter().enumerate() {
      
//             let port_name = if let Ok(port_name) = midi_input.port_name(&p) { port_name } else { "No device name given".to_string() };
    
//             vec_of_devices.push(
//                 MidiDevice{
//                     direction: atoms::input(),
//                     name: port_name,
//                     port: i
//                 });
        
//         }
    
//     });

//     GLOBAL_MIDI_OUTPUT_RESULT.with(|midi_output_result| {

//         let midi_output = match midi_output_result {
//             Ok(midi_device) => midi_device,
//             Err(error) => panic!("Problem getting midi output devices. Error: {:?}", error)
//         };

//         println!("Midi output ports: {:?}\n\r", midi_output.port_count());

//         for (i, p) in midi_output.ports().iter().enumerate() {  
        
//             let port_name = if let Ok(port_name) = midi_output.port_name(&p) { port_name } else { "No device name given".to_string() };
          
//             vec_of_devices.push(
//                 MidiDevice{
//                     direction: atoms::output(),
//                     name: port_name,
//                     port: i
//                 });
//         }

//     });


//     return Ok(vec_of_devices)

// }

// #[rustler::nif(schedule = "DirtyCpu")]
// fn port_count() -> Result<NumPorts, Error>{

//     let mut num_inputs = 0;
//     let mut num_ouputs = 0;

//     GLOBAL_MIDI_INPUT_RESULT.with(|midi_input_result| {

//         // midi_input_result.ignore(Ignore::None);
        
//         let midi_input = match midi_input_result {
//             Ok(midi_device) => midi_device,
//             Err(error) => panic!("Problem getting midi input devices. Attempted creating new connection. Error: {:?}", error)
//         };

//         println!("\nMidi input ports: {:?}\n\r", midi_input.port_count());

//         num_inputs = midi_input.port_count();

//     });

//     GLOBAL_MIDI_OUTPUT_RESULT.with(|midi_output_result| {

//         let midi_output = match midi_output_result {
//             Ok(midi_device) => midi_device,
//             Err(error) => panic!("Problem getting midi output devices. Attempted creating new connection. Error: {:?}", error)
//         };

//         println!("Midi output ports: {:?}\n\r", midi_output.port_count());

//         num_ouputs = midi_output.port_count();

//     });

//     return Ok( NumPorts { input: num_inputs, output: num_ouputs } )

// }





fn on_load(env: Env, _info: Term) -> bool {
    // rustler::resource!(MidiexMidiInputRef<'_>, env);
    // rustler::resource!(MidiexMidiOutputRef, env);
    // rustler::resource!(MidiexMidiInputConnection<T>, env);


    // rustler::resource!(MidiPort, env);
    // rustler::resource!(MidiexMidiInputPortRef, env);
    // rustler::resource!(MidiexMidiOutputPortRef, env);
    // rustler::resource!(MidiexMidiPortRef, env);

    

    // MIDI Input and Output object for the OS
    rustler::resource!(MidiexMidiInputRef, env);
    rustler::resource!(MidiexMidiOutputRef, env);
    
    // MIDI ports (both input and output)
    rustler::resource!(FlexiPort, env);
    rustler::resource!(MidiexMidiSimplePortRef, env);

    // MIDI connection to a MIDI port
    // rustler::resource!(FlexiConn, env);
    // rustler::resource!(MidiexConnRef, env);
    rustler::resource!(OutConnRef, env);


    

    // rustler::resource!(MidexMidiInputConnectionRef, env);
    // rustler::resource!(MidexMidiOutputConnectionRef, env);
    
    true
}

rustler::init!(
    "Elixir.Midiex",
    [count_ports, list_ports, try_connect, try_core_midi, play, play_two, subscribe, connect],
    load = on_load
);