#![windows_subsystem = "windows"]
use notify_rust::Notification;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use image::ImageReader;
use iced::window::icon;
use std::io::Cursor;

use iced::widget::{
    self, button, column, container, row, slider, text, Text
};
use iced::{Element, Center, Fill, Task, Bottom, window};

fn main() -> iced::Result {
    let image_bytes = include_bytes!("../assets/icon_transparent_bg.png");
    let img = ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()
        .expect("Failed to guess image format")
        .decode()
        .unwrap()
        .into_rgba8();
    let width = img.width();
    let height = img.height();
    let icon = icon::from_rgba(img.into_raw(), width, height).unwrap();

    iced::application("Twenty", Twenty::update, Twenty::view)
    .window(window::Settings {
        decorations: true,
        resizable: false,
        size: iced::Size::new(300.0, 300.0),
        icon: Some(icon),
        ..window::Settings::default()
    })
    .run()
}

struct Twenty {
    timeout: u8,
    timer: u8,
    show_modal: bool,
    state: State,
    channel: Arc<Mutex<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
}

#[derive(Debug, Clone)]
enum Message {
    TimerSliderChanged(u8),
    TimeoutSliderChanged(u8),
    ShowModal,
    HandleState
}

#[derive(PartialEq)]
enum State {
    Running,
    Idle
}

impl Twenty {

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TimerSliderChanged(value) => {
                self.timer = value;
                Task::none()
            }
            Message::TimeoutSliderChanged(value) => {
                self.timeout = value;
                Task::none()
            }
            Message::ShowModal => {
                self.show_modal = true;
                widget::focus_next()
            }
            Message::HandleState => {
                match self.state {
                    State::Running => {
                        self.channel.lock().unwrap().0.send(()).unwrap();
                        self.state = State::Idle;
                    }

                    State::Idle => {
                        spawn_timer_thread(self.timeout, self.timer, self.channel.clone());
                        self.state = State::Running;
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let timeout_slider = container(
            slider(5..= 60, self.timeout, Message::TimeoutSliderChanged)
                .default(20)
                .shift_step(1),
        )
        .width(200);

        let timer_slider = container(
            slider(1..= 60, self.timer, Message::TimerSliderChanged)
                .default(20)
                .shift_step(1),
        )
        .width(200);

        let timeout_text: Text = text(format!("Timeout: {} seconds", self.timeout));
        let timer_text = text(format!("Timer: {} minutes", self.timer));
        
        let button_text = match self.state {
            State::Running => "Stop",
            State::Idle => "Start"
        };

        let start_button = button(
                container(
                    text(button_text)
                        .align_x(Center)
                        .width(Fill)
                )
            )
            .on_press(Message::HandleState)
            .width(Fill)
            .padding(10);

        let content = container(
    column![
                row![
                    column![timeout_text, timeout_slider, timer_text, timer_slider]
                        .spacing(20)
                        .padding(20)
                        .width(Fill)
                        .align_x(Center)
                ],

                row![
                    column![start_button]
                        .width(Fill)
                        .align_x(Center)
                ]
                .height(Fill)
                .width(Fill)
                .align_y(Bottom)
            ]
            
        );
        
        content.into()
    }
}

impl Default for Twenty {
    fn default() -> Self {
        Twenty {
            timeout: 20,
            timer: 20,
            show_modal: false,
            state: State::Idle,
            channel: Arc::new(Mutex::new(mpsc::channel())),
        }
    }
}

fn spawn_timer_thread(timeout_arg: u8, timer_arg: u8, channel: Arc<Mutex<(mpsc::Sender<()>, mpsc::Receiver<()>)>>) {
    let timeout = Duration::new(timeout_arg as u64, 0);
    let timer = Duration::new(timer_arg as u64 * 60, 0);

    let channel = Arc::clone(&channel);
    thread::spawn(move || {
        let mut running = true;
        while running {
            let notif_msg = format!("Take a break! Look away from the screen for {} seconds.", timeout.as_secs());
            let mut sleeping: bool = true;
            let start_time = std::time::Instant::now();
            while sleeping {
                match channel.lock().unwrap().1.try_recv() {
                    Ok(a_) => {
                        println!("Received stop message... {:?}", a_);
                        sleeping = false;
                        running = false;
                    }
                    Err(TryRecvError::Disconnected) => {
                        println!("Channel disconnected, stopping timer...");
                        return;
                    }
                    Err(TryRecvError::Empty) => {}
                }
                thread::sleep(Duration::from_millis(300));
                if start_time.elapsed() >= timer {
                    sleeping = false;
                }

            }
            println!("Time to take a break! Look away from the screen.");
            Notification::new()
                .summary("Take a Break!")
                .body(&notif_msg)
                .timeout(timeout.as_secs() as i32)
                .show()
                .unwrap();

            let mut timeouted = true;
            let start_time = std::time::Instant::now();
            while timeouted {
                match channel.lock().unwrap().1.try_recv() {
                    Ok(a_) => {
                        println!("Received stop message... {:?}", a_);
                        timeouted = false;
                        running = false;
                    }
                    Err(TryRecvError::Disconnected) => {
                        println!("Channel disconnected, stopping timer...");
                        return;
                    }
                    Err(TryRecvError::Empty) => {}
                }
                thread::sleep(Duration::from_millis(300));
                if start_time.elapsed() >= timeout {
                    timeouted = false;
                }
            }
        }
    });
}


