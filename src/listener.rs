use log::error;
use rdev::{listen, Event, EventType};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

type Callback = Box<dyn Fn(Event) + Send>;

pub trait EventListener {
    fn listen(&self);
    fn spawn(&self) -> thread::JoinHandle<()>;
    fn add_callback(&mut self, callback: Callback);
}

#[derive(Clone)]
pub struct Listener {
    pub sender: mpsc::Sender<Event>,
    pub receiver: Arc<Mutex<mpsc::Receiver<Event>>>,
    pub callbacks: Arc<Mutex<Vec<Callback>>>,
}

impl Listener {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Listener {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for Listener {
    fn default() -> Self {
        Self::new()
    }
}

impl EventListener for Listener {
    fn listen(&self) {
        let sender = self.sender.clone();
        let callbacks = Arc::clone(&self.callbacks);
        listen(move |event| {
            let callbacks = callbacks.lock().unwrap();
            for callback in callbacks.iter() {
                callback(event.clone());
            }
            sender
                .send(event)
                .unwrap_or_else(|e| error!("Could not send event {:?}", e));
        })
        .expect("Could not listen");
    }

    fn spawn(&self) -> thread::JoinHandle<()> {
        let sender = self.sender.clone();
        let receiver = self.receiver.clone();
        let callbacks = Arc::clone(&self.callbacks);
        thread::spawn(move || {
            let listener = Listener {
                sender,
                receiver,
                callbacks,
            };
            listener.listen();
        })
    }

    fn add_callback(&mut self, callback: Callback) {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.push(callback);
    }
}

pub struct InputListener {
    pub listener: Listener,
    last_mouse_position: Arc<Mutex<Option<(i32, i32)>>>,
    last_click_time: Arc<Mutex<Option<Instant>>>,
    button_pressed: Arc<Mutex<Option<rdev::Button>>>,
    last_button_pressed: Arc<Mutex<Option<rdev::Button>>>,
    keys_pressed: Arc<Mutex<Vec<rdev::Key>>>,
    last_keys_pressed: Arc<Mutex<Vec<rdev::Key>>>,
}

impl Default for InputListener {
    fn default() -> Self {
        Self::new()
    }
}

impl InputListener {
    pub fn new() -> Self {
        let listener = Listener::new();
        InputListener {
            listener,
            last_mouse_position: Arc::new(Mutex::new(None)),
            last_click_time: Arc::new(Mutex::new(None)),
            button_pressed: Arc::new(Mutex::new(None)),
            last_button_pressed: Arc::new(Mutex::new(None)),
            keys_pressed: Arc::new(Mutex::new(Vec::new())),
            last_keys_pressed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_last_mouse_position(&self) -> Option<(i32, i32)> {
        let pos = self.last_mouse_position.lock().unwrap();
        *pos
    }

    pub fn get_last_click_time(&self) -> Option<Instant> {
        let time = self.last_click_time.lock().unwrap();
        *time
    }

    pub fn get_last_button_pressed(&self) -> Option<rdev::Button> {
        let last = self.last_button_pressed.lock().unwrap();
        *last
    }

    pub fn is_button_pressed(&self, button: rdev::Button) -> bool {
        let pressed = self.button_pressed.lock().unwrap();
        match *pressed {
            Some(b) => b == button,
            None => false,
        }
    }

    pub fn is_key_pressed(&self, key: rdev::Key) -> bool {
        let keys = self.keys_pressed.lock().unwrap();
        keys.contains(&key)
    }

    pub fn get_keys_pressed(&self) -> Vec<rdev::Key> {
        let keys = self.keys_pressed.lock().unwrap();
        keys.clone()
    }

    pub fn add_callback(&mut self, callback: Callback) {
        self.listener.add_callback(callback);
    }

    pub fn spawn(&self) -> thread::JoinHandle<()> {
        let sender = self.listener.sender.clone();
        let callbacks = Arc::clone(&self.listener.callbacks);
        let last_mouse_position = Arc::clone(&self.last_mouse_position);
        let last_click_time = Arc::clone(&self.last_click_time);
        let button_pressed = Arc::clone(&self.button_pressed);
        let last_button_pressed = Arc::clone(&self.last_button_pressed);
        let keys_pressed = Arc::clone(&self.keys_pressed);
        let last_keys_pressed = Arc::clone(&self.last_keys_pressed);

        thread::spawn(move || {
            listen(move |event| {
                match event.event_type {
                    EventType::MouseMove { x, y } => {
                        let mut pos = last_mouse_position.lock().unwrap();
                        *pos = Some((x as i32, y as i32));
                    }
                    EventType::ButtonPress(button) => {
                        let mut pressed = button_pressed.lock().unwrap();
                        *pressed = Some(button);
                        let mut time = last_click_time.lock().unwrap();
                        *time = Some(Instant::now());
                        let mut last = last_button_pressed.lock().unwrap();
                        *last = Some(button);
                    }
                    EventType::KeyPress(key) => {
                        let mut keys = keys_pressed.lock().unwrap();
                        keys.push(key);
                        let mut last = last_keys_pressed.lock().unwrap();
                        *last = keys.clone();
                    }
                    EventType::KeyRelease(key) => {
                        let mut keys = keys_pressed.lock().unwrap();
                        keys.retain(|&k| k != key);
                    }
                    EventType::ButtonRelease(_button) => {
                        let mut pressed = button_pressed.lock().unwrap();
                        *pressed = None;
                    }
                    _ => {}
                }

                // Execute all registered callbacks
                let callbacks = callbacks.lock().unwrap();
                for callback in callbacks.iter() {
                    callback(event.clone());
                }

                // Send the event through the channel
                sender
                    .send(event)
                    .unwrap_or_else(|e| error!("Could not send event {:?}", e));
            })
            .expect("Could not listen");
        })
    }
}
