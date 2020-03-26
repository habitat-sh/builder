// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// ZMQ socket address
const INPROC_ADDR: &str = "inproc://logger";
/// ZMQ protocol frame to indicate a log line is being sent
const LOG_LINE: &str = "L";
/// ZMQ protocol frame to indicate a log has finished
const LOG_COMPLETE: &str = "C";
/// End-of-line marker
const EOL_MARKER: &str = "\n";

use std::{fmt,
          io::{BufRead,
               BufReader,
               Read},
          process::Child,
          sync::{Arc,
                 Mutex},
          thread};

use protobuf::Message;
use zmq;

use crate::{bldr_core::{logger::Logger,
                        socket::DEFAULT_CONTEXT},
            protocol::jobsrv::{JobLogChunk,
                               JobLogComplete}};

use super::workspace::Workspace;
use crate::error::{Error,
                   Result};

/// Streams the contents of a Builder job to a remote target. The contents of the stream consist of
/// consuming the output streams of child processes (such as `hab-studio`,
/// `hab-pkg-export-docker`, etc.), section start/end delimiters, and any user-facing error
/// messaging (usually written to a stderr output stream).
///
/// A `JobStreamer` is associated with a Builder job identifer and should be used for the duration
/// of that job.
pub struct JobStreamer {
    /// The job identifer associated with this build log
    id:       u64,
    /// The underlying target for this log when streaming lines. This target may be written to by
    /// multiple concurrent threads, therefore it is managed with a `Mutex`.
    target:   Arc<Mutex<StreamTarget>>,
    /// Whether or not the stream has been marked as finished
    finished: bool,
}

impl JobStreamer {
    /// Constructs a new streamer and writes a job start message to the stream
    ///
    /// # Panics
    ///
    /// * If the stream target mutex is poisoned
    ///
    /// # Errors
    ///
    /// * If the stream target could not be written to
    pub fn new(workspace: &Workspace) -> Self {
        let streamer = JobStreamer { id:       workspace.job.get_id(),
                                     target:   Arc::new(Mutex::new(StreamTarget::new(workspace))),
                                     finished: false, };

        streamer.target
                .lock()
                .expect("Stream target mutex is poisoned!")
                .stream_line(streamer.id, format!("builder_log::start::{}", streamer.id))
                .unwrap();

        streamer
    }

    /// Starts a log section and returns a `LogSection` instance which can be "end"-ed.
    ///
    /// # Errors
    ///
    /// * If the section cannot be started by writing to the stream target
    pub fn start_section(&self, name: Section) -> Result<LogSection> {
        let mut section = LogSection::new(self.id, name, self.target.clone());
        section.start()?;

        Ok(section)
    }

    /// Consumes the `stdout` and `stderr` ouput streams of a child process and writes their
    /// contents to the stream while attempting to preserve the original output ordering.
    ///
    /// Each output stream is independently read from a full line at a time before it writes the
    /// full line to the shared target stream resource. This means that in most cases the output
    /// ordering will be identical to running the same command in a terminal. The exception to this
    /// is if a process is writing to both its `stdout` and `stderr` concurrently, without flushing
    /// their buffers after each newline. Preserving exact ordering did not seem as useful as
    /// preserving ordering at a line level, hence the current implementation.
    ///
    /// # Panics
    ///
    /// * If the child process' `stdout` stream was not captured--this is a programmer error and is
    /// a setup bug
    /// * If the `stdout` consuming thread cannot be spawned--this would most likely happen on a
    /// resource starved system and indicates a possible health issue of the host
    /// * If the child process' `stderr` stream was not captured--this is a programmer error and is
    /// a setup bug
    /// * If the `stderr` consuming thread cannot be spawned--this would most likely happen on a
    /// resource starved system and indicates a possible health issue of the host
    pub fn consume_child(&self, child: &mut Child) -> Result<()> {
        let _stdout_handle = {
            let target = self.target.clone();
            let id = self.id;
            let stdout = child.stdout.take().expect("Child stdout was not captured");
            thread::Builder::new().name("stdout-consumer".into())
                                  .spawn(move || consume_stream(target, id, stdout))
                                  .expect("Failed to spawn stdout thread")
        };
        let _stderr_handle = {
            let target = self.target.clone();
            let id = self.id;
            let stderr = child.stderr.take().expect("Child stderr was not captured");
            thread::Builder::new().name("stderr-consumer".into())
                                  .spawn(move || consume_stream(target, id, stderr))
                                  .expect("Failed to spawn stderr thread")
        };

        Ok(())
    }

    /// Writes a full line from a `stderr` stream to the log stream.
    ///
    /// # Panics
    ///
    /// * If the stream target mutex is poisoned
    ///
    /// # Errors
    ///
    /// * If the stream target could not be written to
    pub fn println_stderr<S: Into<String>>(&self, line: S) -> Result<()> {
        // NOTE fn: Currently in the log file there is no distinction between `stdout` and `stderr`
        // streams. However, if in the future we wish to tag each line with the stream source, this
        // would be where we tag a line as being from a `stderr` source.
        self.target
            .lock()
            .expect("Stream target mutex is poisoned!")
            .stream_line(self.id, line)
    }

    /// Finishes a log streamer by writing any remaining messages, marking the log as complete,
    /// etc. This method can be called multiple times but will only take action once.
    ///
    /// # Panics
    ///
    /// * If the stream target mutex is poisoned
    ///
    /// # Errors
    ///
    /// * If the stream target could not be written to
    pub fn finish(&mut self) -> Result<()> {
        // Early return if the section has ended to make sure that the `Drop` implementation
        // doesn't double-finish the stream.
        if self.finished {
            return Ok(());
        }

        let mut target = self.target
                             .lock()
                             .expect("Stream target mutex is poisoned!");
        target.stream_line(self.id, format!("builder_log::end::{}", self.id))?;
        self.finished = true;
        target.finish(self.id)
    }
}

impl Drop for JobStreamer {
    fn drop(&mut self) {
        // This unwrap is intentional as more error handling isn't possible in a `Drop` trait
        self.finish().unwrap();
    }
}

/// The target to which a log stream is written. This struct wraps a remote socket which is log
/// line aware.
struct StreamTarget {
    /// A zeromq socket which represents the log stream target
    pub sock:         zmq::Socket,
    /// The current line count of submitted log lines
    pub line_count:   u64,
    /// A local file logger that writes a copy of each line written to the remote socket
    pub local_logger: Logger,
}

impl StreamTarget {
    /// Constructs a new stream target with an initialized socket.
    ///
    /// # Panics
    ///
    /// * If the zeromq socket cannot be fully set up
    fn new(workspace: &Workspace) -> Self {
        let sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::PUSH).unwrap();
        sock.set_immediate(true).unwrap();
        sock.set_linger(5000).unwrap();
        sock.connect(INPROC_ADDR).unwrap();

        let id = workspace.job.get_id().to_string();
        let mut local_logger = Logger::init(workspace.root(), format!("local-stream-{}.log", &id));
        local_logger.log_ident(&id);

        StreamTarget { sock,
                       line_count: 0,
                       local_logger }
    }

    /// Takes a string, interpreted as a single line, with a job identifier and writes it to the
    /// log stream on the socket.
    ///
    /// # Panics
    ///
    /// * If the protobuf struct cannot be serialized into bytes
    ///
    /// # Errors
    ///
    /// * If a message couldn't be sent successfully to the stream target socket
    fn stream_line<S: Into<String>>(&mut self, id: u64, line: S) -> Result<()> {
        let mut line: String = line.into();
        self.local_logger.log(&line);
        line.push_str(EOL_MARKER);

        self.line_count += 1;

        let mut chunk = JobLogChunk::new();
        chunk.set_job_id(id);
        chunk.set_seq(self.line_count);
        chunk.set_content(line);

        self.sock
            .send_str(LOG_LINE, zmq::SNDMORE)
            .map_err(Error::StreamTargetSend)?;
        self.sock
            .send(chunk.write_to_bytes().unwrap().as_slice(), 0)
            .map_err(Error::StreamTargetSend)?;

        Ok(())
    }

    /// Marks the log stream as completed using the job identifier.
    ///
    /// # Panics
    ///
    /// * If the protobuf struct cannot be serialized into bytes
    ///
    /// # Errors
    ///
    /// * If a message couldn't be sent successfully to the stream target socket
    fn finish(&mut self, id: u64) -> Result<()> {
        let mut complete = JobLogComplete::new();
        complete.set_job_id(id);

        self.sock
            .send_str(LOG_COMPLETE, zmq::SNDMORE)
            .map_err(Error::StreamTargetSend)?;
        self.sock
            .send(complete.write_to_bytes().unwrap().as_slice(), 0)
            .map_err(Error::StreamTargetSend)?;

        Ok(())
    }
}

/// A controlled name set of section names in a job log. These section names may be output to a
/// Builder user via job output.
pub enum Section {
    BuildPackage,
    CloneRepository,
    ExportDocker,
    FetchOriginKey,
    PublishPackage,
    ValidateIntegrations,
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // The token output should ideally be lowercased, underscore-delimited, and present-tense
        // verb-leading for consistency
        let token = match *self {
            Section::BuildPackage => "build_package",
            Section::CloneRepository => "clone_repository",
            Section::ExportDocker => "export_docker",
            Section::FetchOriginKey => "fetch_origin_key",
            Section::PublishPackage => "publish_package",
            Section::ValidateIntegrations => "validate_integrations",
        };
        write!(f, "{}", token)
    }
}

/// A section of a job log. A section typically maps to a build task such as cloning a repository,
/// publishing and artifact, building a packages, etc. Note that this may also correspond to
/// certain failure scenarios such as being unable to clone a repository, failing to build a
/// package in the Studio, etc.
pub struct LogSection {
    /// The job identifer associated with this build log
    id:     u64,
    /// The section name
    name:   Section,
    /// The underlying target for this log when streaming lines
    target: Arc<Mutex<StreamTarget>>,
    /// Whether or not the section has been marked as ended
    ended:  bool,
}

impl LogSection {
    /// Constructs a new log section which has not yet been started.
    fn new(id: u64, name: Section, target: Arc<Mutex<StreamTarget>>) -> Self {
        LogSection { id,
                     name,
                     target,
                     ended: false }
    }

    /// Starts a log section by writing to the log stream.
    ///
    /// # Panics
    ///
    /// * If the stream target mutex is poisoned
    ///
    /// # Errors
    ///
    /// * If the stream target could not be written to
    fn start(&mut self) -> Result<()> {
        self.target
            .lock()
            .expect("Stream target mutex is poisoned!")
            .stream_line(self.id,
                         format!("builder_log_section::start::{}", self.name))
    }

    /// Ends a log section by writing to the log stream. This method can be called multiple times
    /// but will only take action once.
    ///
    /// # Panics
    ///
    /// * If the stream target mutex is poisoned
    ///
    /// # Errors
    ///
    /// * If the stream target could not be written to
    pub fn end(&mut self) -> Result<()> {
        // Early return if the section has ended to make sure that the `Drop` implementation
        // doesn't double-close the section.
        if self.ended {
            return Ok(());
        }

        self.ended = true;
        self.target
            .lock()
            .expect("Stream target mutex is poisoned!")
            .stream_line(self.id, format!("builder_log_section::end::{}", self.name))
    }
}

impl Drop for LogSection {
    fn drop(&mut self) {
        // This unwrap is intentional as more error handling isn't possible in a `Drop` trait
        self.end().unwrap();
    }
}

/// Takes a `Read`er with an identifier and writes its contents to a stream target, one line at a
/// time.
///
/// # Panics
///
/// * If there is an error reading a single line from the reader stream
/// * If the stream target mutex is poisoned
///
/// # Errors
///
/// * If the stream target could not be written to
#[allow(clippy::needless_pass_by_value)]
fn consume_stream<R: Read>(target: Arc<Mutex<StreamTarget>>, id: u64, reader: R) -> Result<()> {
    let reader = BufReader::new(reader);
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => return Err(Error::StreamLine(e)),
        };
        target.lock()
              .expect("Stream target mutex is poisoned!")
              .stream_line(id, line)?;
    }

    Ok(())
}
