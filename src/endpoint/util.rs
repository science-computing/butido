//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::sync::Arc;

use anyhow::Result;
use futures::FutureExt;
use tokio_stream::StreamExt;

use crate::endpoint::Endpoint;
use crate::endpoint::EndpointConfiguration;

pub async fn setup_endpoints(endpoints: Vec<EndpointConfiguration>) -> Result<Vec<Arc<Endpoint>>> {
    let unordered = futures::stream::FuturesUnordered::new();

    for cfg in endpoints.into_iter() {
        unordered
            .push(Endpoint::setup(cfg).map(|r_ep| r_ep.map(Arc::new)));
    }

    unordered.collect().await
}

