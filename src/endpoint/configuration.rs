//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::Getters;
use typed_builder::TypedBuilder;

use crate::util::docker::ImageName;

#[derive(Getters, TypedBuilder)]
pub struct EndpointConfiguration {
    #[getset(get = "pub")]
    endpoint_name: crate::config::EndpointName,

    #[getset(get = "pub")]
    endpoint: crate::config::Endpoint,

    #[getset(get = "pub")]
    #[builder(default)]
    required_images: Vec<ImageName>,

    #[getset(get = "pub")]
    #[builder(default)]
    required_docker_versions: Option<Vec<String>>,

    #[getset(get = "pub")]
    #[builder(default)]
    required_docker_api_versions: Option<Vec<String>>,
}
