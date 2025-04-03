use crate::hab_core::env;
use dogstatsd::{Client,
                Options};
use std::{borrow::{Borrow,
                   Cow},
          sync::{mpsc::{channel,
                        sync_channel,
                        Receiver,
                        Sender,
                        SyncSender},
                 Mutex},
          thread};

// Statsd Application name
pub const APP_NAME: &str = "bldr";

// Statsd Listener Address
pub const STATS_ENV: &str = "HAB_STATS_ADDR";

pub type InstallationId = u32;

// Public Interface
////////////////////////////////////////////////////////////////////////

/// Metric identifiers will usually be static `str`s, but some may
/// need to be dynamically-generated `String`s. With a `Cow`, we can
/// accept either.
pub type MetricId = Cow<'static, str>;

/// All metrics must implement the Metric trait, as well as one of the
/// type marker traits (e.g., `CounterMetric`).
pub trait Metric {
    /// Generate the metric name to be used
    fn id(&self) -> MetricId;
}

pub trait CounterMetric: Metric {
    /// Increment the metric by one
    fn increment(&self) {
        match sender().send((MetricType::Counter,
                             MetricOperation::Increment,
                             self.id(),
                             None,
                             vec![]))
        {
            Ok(_) => (),
            Err(e) => error!("Failed to increment counter, error: {:?}", e),
        }
    }
}

pub trait HistogramMetric: Metric {
    /// Set the value of the gauge
    fn set(&self, val: MetricValue) {
        match sender().send((MetricType::Histogram,
                             MetricOperation::Set,
                             self.id(),
                             Some(val),
                             vec![]))
        {
            Ok(_) => (),
            Err(e) => error!("Failed to set gauge, error: {:?}", e),
        }
    }
}

// Implementation Details
////////////////////////////////////////////////////////////////////////////////

// Helper types
#[derive(Debug, Clone, Copy)]
enum MetricType {
    Counter,
    Histogram,
}

#[derive(Debug, Clone, Copy)]
enum MetricOperation {
    Increment,
    Set,
}

type MetricValue = f64;
type MetricTuple = (MetricType, MetricOperation, MetricId, Option<MetricValue>, Vec<String>);

// One-time initialization
lazy_static! {
    // Assuming it's safe to have lazy static start thread
    static ref SENDER: Mutex<Sender<MetricTuple>> = Mutex::new(init());
}

fn sender() -> Sender<MetricTuple> {
    // Assuming it's safe to have lazy static start thread
    // As best as I can determine, this will acquire and release two locks
    // * acquire lock around the lazy static SENDER, specifically:
    // * if it is not initialized, init
    // * release that lock now that we have the initialized value
    // * acquire lock for SENDER mutex,
    // * clone SENDER
    // * release mutex
    // A cleverer implementation could probably avoid one of those locks, but that would
    // probably require some care to be correct.
    (*SENDER).lock().unwrap().clone()
}

// init creates a worker thread ready to receive and process metric events,
// and returns a channel for use by metric senders
fn init() -> Sender<MetricTuple> {
    let (tx, rx) = channel::<MetricTuple>();
    let (rztx, rzrx) = sync_channel(0); // rendezvous channel

    thread::Builder::new().name("metrics".to_string())
                          .spawn(move || receive(&rztx, &rx))
                          .expect("couldn't start metrics thread");

    match rzrx.recv() {
        Ok(()) => tx,
        Err(e) => panic!("metrics thread startup error, err={}", e),
    }
}

// receive runs in a separate thread and processes all metrics events
fn receive(rz: &SyncSender<()>, rx: &Receiver<MetricTuple>) {
    let mut client = statsd_client();
    rz.send(()).unwrap(); // Blocks until the matching receive is called

    loop {
        let (mtyp, mop, mid, mval, mtags): MetricTuple = rx.recv().unwrap();

        if let Some(ref mut cli) = client {
            match mtyp {
                MetricType::Counter => {
                    match mop {
                        MetricOperation::Increment => {
                            let mid_str: &str = mid.borrow();
                            cli.incr(mid_str, &mtags)
                               .unwrap_or_else(|e| warn!("Could not increment metric; {:?}", e))
                        }
                        _ => warn!("Unexpected metric operation!"),
                    };
                }
                MetricType::Histogram => {
                    match mop {
                        MetricOperation::Set => {
                            let mid_str: &str = mid.borrow();
                            let val_str = format!("{}", mval.unwrap());
                            cli.histogram(mid_str, val_str, &mtags).unwrap_or_else(|e| {
                                                                       warn!("Could not set \
                                                                              metric; {:?}",
                                                                             e)
                                                                   })
                        }
                        _ => warn!("Unexpected metric operation!"),
                    }
                }
            }
        }
    }
}

fn statsd_client() -> Option<Client> {
    match env::var(STATS_ENV) {
        Ok(addr) => {
            info!("Creating DogStatsD client sending to: {}", addr);

            // Bind to an arbitrary UDP port for sending; this is what
            // the old statsd client we were using does, but the
            // DogStatsD client exposes this as a parameter.
            let options = Options::new("0.0.0.0:0", &addr, APP_NAME);
            match Client::new(options) {
                Ok(c) => Some(c),
                Err(e) => {
                    error!("Error creating statsd client: {:?}", e);
                    None
                }
            }
        }
        Err(_) => None,
    }
}
