use std::future::Future;
use std::pin::Pin;
use std::string::ToString;
use std::task::{Context, Poll};

use strum::Display;
use tokio::sync::mpsc::UnboundedSender;

pub trait Signaller: Unpin + 'static {
    fn signal(&self, sig: Signal);
}

pub trait FromSignal {
    fn from(sig: Signal) -> Self;
}

impl<T: FromSignal + 'static> Signaller for UnboundedSender<T> {
    fn signal(&self, sig: Signal) {
        let _ = self.send(T::from(sig));
    }
}

/// Different types of process signals
#[allow(dead_code)]
#[derive(PartialEq, Clone, Copy, Debug, Display)]
pub enum Signal {
    /// SIGHUP
    Hup,
    /// SIGINT
    Int,
    /// SIGTERM
    Term,
    /// SIGQUIT
    Quit,
}

impl FromSignal for Signal {
    fn from(sig: Signal) -> Self {
        sig
    }
}

pub struct OneshotSignals<S: Signaller> {
    signaller: S,
    #[cfg(not(unix))]
    signals: futures_util::future::LocalBoxFuture<'static, std::io::Result<()>>,
    #[cfg(unix)]
    signals: Vec<(Signal, actix_rt::signal::unix::Signal)>,
}

impl<S: Signaller> OneshotSignals<S> {
    pub fn start(signaller: S) {
        #[cfg(not(unix))]
        {
            actix_rt::spawn(OneshotSignals {
                signaller,
                signals: Box::pin(actix_rt::signal::ctrl_c()),
            });
        }
        #[cfg(unix)]
        {
            use actix_rt::signal::unix;

            let sig_map = [
                (unix::SignalKind::interrupt(), Signal::Int),
                (unix::SignalKind::hangup(), Signal::Hup),
                (unix::SignalKind::terminate(), Signal::Term),
                (unix::SignalKind::quit(), Signal::Quit),
            ];

            let signals = sig_map
                .iter()
                .filter_map(|(kind, sig)| {
                    unix::signal(*kind)
                        .map(|tokio_sig| (*sig, tokio_sig))
                        .map_err(|e| {
                            log::error!(
                                "Can not initialize stream handler for {:?} err: {}",
                                sig,
                                e
                            )
                        })
                        .ok()
                })
                .collect::<Vec<_>>();

            actix_rt::spawn(OneshotSignals { signaller, signals });
        }
    }
}

impl<S: Signaller> Future for OneshotSignals<S> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[cfg(not(unix))]
        match self.signals.as_mut().poll(cx) {
            Poll::Ready(_) => {
                self.signaller.signal(Signal::Int);
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
        #[cfg(unix)]
        {
            for (sig, fut) in self.signals.iter_mut() {
                if Pin::new(fut).poll_recv(cx).is_ready() {
                    let sig = *sig;
                    self.signaller.signal(sig);
                    return Poll::Ready(());
                }
            }
            Poll::Pending
        }
    }
}
