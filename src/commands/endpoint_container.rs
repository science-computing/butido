//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'endpoint container' subcommand

use std::borrow::Cow;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use tokio_stream::StreamExt;

use crate::config::Configuration;
use crate::config::EndpointName;
use crate::endpoint::Container;

pub async fn container(
    endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let container_id = matches.get_one::<String>("container_id").unwrap();
    let endpoints =
        crate::commands::endpoint::connect_to_endpoints(config, &endpoint_names).await?;
    let relevant_endpoints = endpoints
        .into_iter()
        .map(|ep| async {
            ep.has_container_with_id(container_id)
                .await
                .map(|b| (ep, b))
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<(_, bool)>>>()
        .await?
        .into_iter()
        .filter_map(|tpl| if tpl.1 { Some(tpl.0) } else { None })
        .collect::<Vec<_>>();

    if relevant_endpoints.len() > 1 {
        anyhow::bail!("Found more than one container for id {container_id}");
    }

    let relevant_endpoint = relevant_endpoints
        .first()
        .ok_or_else(|| anyhow!("Found no container for id {container_id}"))?;

    let container = relevant_endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "Cannot find container {} on {}",
                container_id,
                relevant_endpoint.name()
            )
        })?;

    let confirm = |prompt: String| dialoguer::Confirm::new().with_prompt(prompt).interact();

    match matches.subcommand() {
        Some(("top", matches)) => top(matches, container).await,
        Some(("kill", matches)) => {
            confirm({
                if let Some(sig) = matches.get_one::<String>("signal") {
                    format!("Really kill {container_id} with {sig}?")
                } else {
                    format!("Really kill {container_id}?")
                }
            })?;

            kill(matches, container).await
        }
        Some(("delete", _)) => {
            if confirm(format!("Really delete {container_id}?"))? {
                delete(container).await
            } else {
                Ok(())
            }
        }
        Some(("start", _)) => {
            if confirm(format!("Really start {container_id}?"))? {
                start(container).await
            } else {
                Ok(())
            }
        }
        Some(("stop", matches)) => {
            if confirm(format!("Really stop {container_id}?"))? {
                stop(matches, container).await
            } else {
                Ok(())
            }
        }
        Some(("exec", matches)) => {
            let commands = matches
                .get_many::<String>("commands")
                .unwrap()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>();
            if confirm(format!(
                "Really run '{}' in {}?",
                commands.join(" "),
                container_id
            ))? {
                exec(matches, container).await
            } else {
                Ok(())
            }
        }
        Some(("inspect", _)) => inspect(container).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {other}")),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn top(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    let top = container.top(None).await?;
    let hdr = crate::commands::util::mk_header(top.titles.iter().map(|s| s.as_str()).collect());
    crate::commands::util::display_data(hdr, top.processes, matches.get_flag("csv"))
}

async fn kill(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    let signal = matches.get_one::<String>("signal");
    container.kill(signal.map(|s| s.as_str())).await
}

async fn delete(container: Container<'_>) -> Result<()> {
    container.delete().await
}

async fn start(container: Container<'_>) -> Result<()> {
    container.start().await
}

async fn stop(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    container
        .stop({
            matches
                .get_one::<u64>("timeout")
                .map(|t| std::time::Duration::from_secs(*t))
        })
        .await
}

async fn exec(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    use std::io::Write;

    let commands = matches
        .get_many::<String>("commands")
        .unwrap()
        .cloned()
        .collect::<Vec<String>>();

    let mut stream = container.exec(commands, true, true).await?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        match chunk {
            bollard::container::LogOutput::StdIn { .. } => {
                return Err(anyhow!("Cannot handle STDIN TTY chunk"));
            }
            bollard::container::LogOutput::StdOut { message } => {
                std::io::stdout().write_all(&message).map_err(Error::from)?;
            }
            bollard::container::LogOutput::StdErr { message } => {
                std::io::stderr().write_all(&message).map_err(Error::from)?;
            }
            bollard::container::LogOutput::Console { message } => {
                std::io::stdout().write_all(&message).map_err(Error::from)?;
            }
        }
    }
    Ok(())
}

// Print inspect details about the container
//
//
//  ABANDON HOPE ALL YE WHO ENTER HERE
//
// (Dante)
//
// This is the most ugly function of the whole codebase. As ugly as it is: It is simply printing
// things, nothing here is too complex code-wise (except some nested formatting stuff...)
async fn inspect(container: Container<'_>) -> Result<()> {
    use itertools::Itertools;
    use std::io::Write;

    let d = container.inspect().await?;

    fn option_vec<'a>(ov: Option<&Vec<String>>) -> Cow<'a, str> {
        ov.map(|v| format!("Some({})", v.iter().join(", ")))
            .map(Cow::from)
            .unwrap_or_else(|| Cow::from("None"))
    }

    fn option_vec_nl<'a>(ov: Option<&Vec<String>>, ind: usize) -> Cow<'a, str> {
        ov.map(|v| {
            v.iter()
                .map(|s| format!("{:ind$}{s}", "", ind = ind, s = s))
                .join("\n")
        })
        .map(|s| format!("\n{s}"))
        .map(Cow::from)
        .unwrap_or_else(|| Cow::from("None"))
    }

    fn option_tostr<'a, T: ToString + 'a>(ots: Option<T>) -> Cow<'a, str> {
        ots.map(|s| format!("Some({})", s.to_string()))
            .map(Cow::from)
            .unwrap_or_else(|| Cow::from("None"))
    }

    let config = d.config.unwrap_or_default();
    let host_config = d.host_config.unwrap_or_default();
    let network_settings = d.network_settings.unwrap_or_default();
    let state = d.state.unwrap_or_default();
    let default_net = network_settings
        .networks
        .as_ref()
        .and_then(|nets| nets.get("bridge").or_else(|| nets.values().next()));

    writeln!(
        std::io::stdout(),
        "{}",
        indoc::formatdoc!(
            r#"
        Container: {container_id}

        app_armor_profile: {app_armor_profile}
        args: {args}
        config:
            attach_stderr: {config_attach_stderr}
            attach_stdin: {config_attach_stdin}
            attach_stdout: {config_attach_stdout}
            cmd: {config_cmd}
            domainname: {config_domainname}
            entrypoint: {config_entrypoint}
            env: {config_env}
            exposed_ports: {config_exposed_ports}
            hostname: {config_hostname}
            image: {config_image}
            labels: {config_labels}
            on_build: {config_on_build}
            open_stdin: {config_open_stdin}
            stdin_once: {config_stdin_once}
            tty: {config_tty}
            user: {config_user}
            working_dir: {config_working_dir}

        created: {created}
        driver: {driver}
        host_config:
            cgroup_parent: {host_config_cgroup_parent}
            container_id_file: {host_config_container_id_file}
            cpu_shares: {host_config_cpu_shares}
            cpuset_cpus: {host_config_cpuset_cpus}
            memory: {host_config_memory}
            memory_swap: {host_config_memory_swap}
            network_mode: {host_config_network_mode}
            pid_mode: {host_config_pid_mode}
            port_bindings: {host_config_port_bindings}
            privileged: {host_config_privileged}
            publish_all_ports: {host_config_publish_all_ports}
            readonly_rootfs: {host_config_readonly_rootfs}
        hostname_path: {hostname_path}
        hosts_path: {hosts_path}
        log_path: {log_path}
        id: {id}
        image: {image}
        mount_label: {mount_label}
        name: {name}
        network_settings:
            bridge: {network_settings_bridge}
            gateway: {network_settings_gateway}
            ip_address: {network_settings_ip_address}
            ip_prefix_len: {network_settings_ip_prefix_len}
            mac_address: {network_settings_mac_address}
            ports: {network_settings_ports}
            networks: {network_settings_networks}
        path: {path}
        process_label: {process_label}
        resolv_conf_path: {resolv_conf_path}
        restart_count: {restart_count}
        state:
            error: {state_error}
            exit_code: {state_exit_code}
            finished_at: {state_finished_at}
            oom_killed: {state_oom_killed}
            paused: {state_paused}
            pid: {state_pid}
            restarting: {state_restarting}
            running: {state_running}
            started_at: {state_started_at}
            status: {state_status}
        mounts: {mounts}
    "#,
            container_id = container.id(),
            app_armor_profile = d.app_armor_profile.unwrap_or_default(),
            args = d
                .args
                .as_ref()
                .map(|a| a.iter().join(", "))
                .unwrap_or_default(),
            config_attach_stderr = config.attach_stderr.unwrap_or_default().to_string(),
            config_attach_stdin = config.attach_stdin.unwrap_or_default().to_string(),
            config_attach_stdout = config.attach_stdout.unwrap_or_default().to_string(),
            config_cmd = option_vec(config.cmd.as_ref()),
            config_domainname = config.domainname.unwrap_or_default(),
            config_entrypoint = option_vec(config.entrypoint.as_ref()),
            config_env = option_vec_nl(config.env.as_ref(), 8),
            config_exposed_ports = {
                config
                    .exposed_ports
                    .as_ref()
                    .map(|v| {
                        let s = v
                            .iter()
                            .map(|k| format!("{:ind$}{k}:", "", ind = 8))
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("\n{s}")
                    })
                    .unwrap_or_else(|| String::from("None"))
            },
            config_hostname = config.hostname.unwrap_or_default(),
            config_image = config.image.unwrap_or_default(),
            config_labels = {
                config
                    .labels
                    .as_ref()
                    .map(|hm| {
                        let s = hm
                            .iter()
                            .map(|(k, v)| format!("{:ind$}{k}: {v}", "", ind = 8, k = k, v = v))
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("\n{s}")
                    })
                    .unwrap_or_else(|| String::from("None"))
            },
            config_on_build = option_vec(config.on_build.as_ref()),
            config_open_stdin = config.open_stdin.unwrap_or_default().to_string(),
            config_stdin_once = config.stdin_once.unwrap_or_default().to_string(),
            config_tty = config.tty.unwrap_or_default().to_string(),
            config_user = config.user.unwrap_or_default(),
            config_working_dir = config.working_dir.unwrap_or_default(),
            created = d.created.map(|c| c.to_string()).unwrap_or_default(),
            driver = d.driver.unwrap_or_default(),
            host_config_cgroup_parent = option_tostr(host_config.cgroup_parent.as_ref()),
            host_config_container_id_file = host_config.container_id_file.unwrap_or_default(),
            host_config_cpu_shares = option_tostr(host_config.cpu_shares.as_ref()),
            host_config_cpuset_cpus = option_tostr(host_config.cpuset_cpus.as_ref()),
            host_config_memory = option_tostr(host_config.memory.as_ref()),
            host_config_memory_swap = option_tostr(host_config.memory_swap.as_ref()),
            host_config_network_mode = host_config.network_mode.unwrap_or_default(),
            host_config_pid_mode = option_tostr(host_config.pid_mode.as_ref()),
            host_config_port_bindings = {
                host_config
                    .port_bindings
                    .as_ref()
                    .map(|hm| {
                        let s = hm
                            .iter()
                            .map(|(k, v)| {
                                let v = v
                                    .as_ref()
                                    .map(|pbs| {
                                        pbs.iter()
                                            .map(|pb| {
                                                format!(
                                                    "{:ind$}HostIp: {}\n{:ind$}HostPort: {}",
                                                    "",
                                                    pb.host_ip.as_deref().unwrap_or(""),
                                                    "",
                                                    pb.host_port.as_deref().unwrap_or(""),
                                                    ind = 12
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    })
                                    .unwrap_or_default();

                                format!("{:ind$}{k}: \n{v}", "", ind = 8, k = k, v = v)
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("\n{s}")
                    })
                    .unwrap_or_else(|| String::from("None"))
            },
            host_config_privileged = host_config.privileged.unwrap_or_default().to_string(),
            host_config_publish_all_ports = host_config
                .publish_all_ports
                .unwrap_or_default()
                .to_string(),
            host_config_readonly_rootfs = option_tostr(host_config.readonly_rootfs.as_ref()),
            hostname_path = d.hostname_path.unwrap_or_default(),
            hosts_path = d.hosts_path.unwrap_or_default(),
            log_path = d.log_path.unwrap_or_default(),
            id = d.id.unwrap_or_default(),
            image = d.image.unwrap_or_default(),
            mount_label = d.mount_label.unwrap_or_default(),
            name = d.name.unwrap_or_default(),
            network_settings_bridge = String::new(),
            network_settings_gateway = default_net
                .and_then(|n| n.gateway.clone())
                .unwrap_or_default(),
            network_settings_ip_address = default_net
                .and_then(|n| n.ip_address.clone())
                .unwrap_or_default(),
            network_settings_ip_prefix_len = default_net
                .and_then(|n| n.ip_prefix_len)
                .map(|l| l.to_string())
                .unwrap_or_else(|| "0".to_string()),
            network_settings_mac_address = default_net
                .and_then(|n| n.mac_address.clone())
                .unwrap_or_default(),
            network_settings_ports = {
                network_settings
                    .ports
                    .as_ref()
                    .map(|hm| {
                        let s = hm
                            .iter()
                            .map(|(k, v)| {
                                let v = v
                                    .as_ref()
                                    .map(|pbs| {
                                        pbs.iter()
                                            .map(|pb| {
                                                format!(
                                                    "{:ind$}HostIp: {}\n{:ind$}HostPort: {}",
                                                    "",
                                                    pb.host_ip.as_deref().unwrap_or(""),
                                                    "",
                                                    pb.host_port.as_deref().unwrap_or(""),
                                                    ind = 12
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    })
                                    .unwrap_or_default();

                                format!("{:ind$}{k}: \n{v}", "", ind = 8, k = k, v = v)
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("\n{s}")
                    })
                    .unwrap_or_else(|| String::from("None"))
            },
            network_settings_networks = {
                network_settings
                    .networks
                    .as_ref()
                    .map(|nets| {
                        nets.iter()
                            .map(|(k, v)| {
                                indoc::formatdoc!(
                                    r#"
                        {k}:
                            network_id: {network_id}
                            endpoint_id: {endpoint_id}
                            gateway: {gateway}
                            ip_address: {ip_address}
                            ip_prefix_len: {ip_prefix_len}
                            ipv6_gateway: {ipv6_gateway}
                            global_ipv6_address: {global_ipv6_address}
                            global_ipv6_prefix_len: {global_ipv6_prefix_len}
                            mac_address: {mac_address}
                    "#,
                                    k = k,
                                    network_id = v.network_id.as_deref().unwrap_or(""),
                                    endpoint_id = v.endpoint_id.as_deref().unwrap_or(""),
                                    gateway = v.gateway.as_deref().unwrap_or(""),
                                    ip_address = v.ip_address.as_deref().unwrap_or(""),
                                    ip_prefix_len =
                                        v.ip_prefix_len.map(|p| p.to_string()).unwrap_or_default(),
                                    ipv6_gateway = v.ipv6_gateway.as_deref().unwrap_or(""),
                                    global_ipv6_address =
                                        v.global_ipv6_address.as_deref().unwrap_or(""),
                                    global_ipv6_prefix_len = v
                                        .global_ipv6_prefix_len
                                        .map(|p| p.to_string())
                                        .unwrap_or_default(),
                                    mac_address = v.mac_address.as_deref().unwrap_or(""),
                                )
                                .lines()
                                .map(|s| format!("{:ind$}{s}", "", ind = 8, s = s))
                                .join("\n")
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .map(|s| format!("\n{s}"))
                    .unwrap_or_else(|| String::from("None"))
            },
            path = d.path.unwrap_or_default(),
            process_label = d.process_label.unwrap_or_default(),
            resolv_conf_path = d.resolv_conf_path.unwrap_or_default(),
            restart_count = d.restart_count.unwrap_or_default().to_string(),
            state_error = state.error.unwrap_or_default(),
            state_exit_code = state.exit_code.unwrap_or_default().to_string(),
            state_finished_at = state.finished_at.unwrap_or_default(),
            state_oom_killed = state.oom_killed.unwrap_or_default().to_string(),
            state_paused = state.paused.unwrap_or_default().to_string(),
            state_pid = state.pid.unwrap_or_default().to_string(),
            state_restarting = state.restarting.unwrap_or_default().to_string(),
            state_running = state.running.unwrap_or_default().to_string(),
            state_started_at = state.started_at.unwrap_or_default(),
            state_status = state.status.map(|s| s.to_string()).unwrap_or_default(),
            mounts = {
                d.mounts
                    .as_ref()
                    .map(|mounts| {
                        mounts
                            .iter()
                            .map(|mount| {
                                indoc::formatdoc!(
                                    r#"
                            source: {source}
                            destination: {destination}
                            mode: {mode}
                            rw: {rw}

                        "#,
                                    source = mount.source.as_deref().unwrap_or(""),
                                    destination = mount.destination.as_deref().unwrap_or(""),
                                    mode = mount.mode.as_deref().unwrap_or(""),
                                    rw = mount.rw.unwrap_or_default().to_string()
                                )
                                .lines()
                                .map(|s| format!("{:ind$}{s}", "", ind = 4, s = s))
                                .join("\n")
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .map(|s| format!("\n{s}"))
                    .unwrap_or_else(|| String::from("None"))
            }
        )
    )
    .map_err(Error::from)
}
