use hulk::signals::{FromSignal, OneshotSignals, Signal};
use log::info;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub enum Event {
    ServerStop,
    Signal(Signal),
}

type EventSender = UnboundedSender<Event>;
type EventReceiver = UnboundedReceiver<Event>;

pub struct EventHandler {
    tx: EventSender,
    rx: EventReceiver,
}

impl EventHandler {
    pub fn new() -> EventHandler {
        let (tx, rx) = unbounded_channel();
        EventHandler { tx, rx }
    }

    pub fn sender(&self) -> EventSender {
        self.tx.clone()
    }

    pub async fn handle_events(&mut self) {
        OneshotSignals::start(self.tx.clone());

        while let Some(event) = self.rx.recv().await {
            match event {
                Event::Signal(signal) => {
                    use Signal::*;
                    match signal {
                        Int | Term | Quit => {
                            info!("Exiting on signal: {}", signal);
                            exit(stop_process());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

impl FromSignal for Event {
    fn from(sig: Signal) -> Self {
        Event::Signal(sig)
    }
}

fn exit(success: bool) {
    std::process::exit(if success { 0 } else { 1 });
}

fn stop_process() -> bool {
    true // TODO
}
