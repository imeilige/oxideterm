// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

//! Headless audit writer ownership.
//!
//! Every crate that records an operation (SSH connections, port forwards,
//! terminal sessions, SFTP transfers, credential and settings changes) writes
//! through `oxideterm_audit::AuditContext`. The global context is only
//! installed while an `AuditService` owns the SQLite writer, so this runtime
//! holds that ownership for the whole workspace lifetime even though the audit
//! viewer UI no longer exists.

use std::path::PathBuf;

use oxideterm_audit::{AuditContext, AuditRegistration, AuditService, AuditSource};

pub(in crate::workspace) struct AuditRuntime {
    /// Keeps the SQLite writer alive; dropping it stops audit persistence.
    _service: Option<AuditService>,
    /// Installs the process-wide context that producers write through.
    _registration: Option<AuditRegistration>,
}

impl AuditRuntime {
    /// Starts the audit writer and installs the global audit context.
    ///
    /// A failure to open the database degrades auditing only: the application
    /// still starts, it just records no audit trail.
    pub(in crate::workspace) fn start(path: PathBuf) -> Self {
        let service = match AuditService::start(path) {
            Ok(service) => Some(service),
            Err(_) => None,
        };
        let registration = service.as_ref().map(|service| {
            AuditContext::new(
                service.client().with_source(AuditSource::User),
                AuditSource::Application,
            )
            .install()
        });
        Self {
            _service: service,
            _registration: registration,
        }
    }
}
