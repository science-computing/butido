//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'endpoint container' subcommand

use std::str::FromStr;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use tokio_stream::StreamExt;
use shiplift::Container;

use crate::config::Configuration;
use crate::config::EndpointName;

pub async fn container(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let container_id = matches.value_of("container_id").unwrap();
    let endpoints = crate::commands::endpoint::connect_to_endpoints(config, &endpoint_names).await?;
    let relevant_endpoints = endpoints.into_iter()
        .map(|ep| async {
            ep.has_container_with_id(container_id)
                .await
                .map(|b| (ep, b))
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<(_, bool)>>>()
        .await?
        .into_iter()
        .filter_map(|tpl| {
            if tpl.1 {
                Some(tpl.0)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if relevant_endpoints.len() > 1 {
        return Err(anyhow!("Found more than one container for id {}", container_id))
    }

    let relevant_endpoint = relevant_endpoints.get(0).ok_or_else(|| {
        anyhow!("Found no container for id {}", container_id)
    })?;

    let container = relevant_endpoint.get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, relevant_endpoint.name()))?;

    let confirm = |prompt: String| dialoguer::Confirm::new().with_prompt(prompt).interact();

    match matches.subcommand() {
        Some(("top", matches))  => top(matches, container).await,
        Some(("kill", matches)) => {
            confirm({
                if let Some(sig) = matches.value_of("signal").as_ref() {
                    format!("Really kill {} with {}?", container_id, sig)
                } else {
                    format!("Really kill {}?", container_id)
                }
            })?;

            kill(matches, container).await
        },
        Some(("delete", _)) => {
            if confirm(format!("Really delete {}?", container_id))? {
                delete(container).await
            } else {
                Ok(())
            }
        },
        Some(("start", _))      => {
            if confirm(format!("Really start {}?", container_id))? {
                start(container).await
            } else {
                Ok(())
            }
        },
        Some(("stop", matches)) => {
            if confirm(format!("Really stop {}?", container_id))? {
                stop(matches, container).await
            } else {
                Ok(())
            }
        },
        Some(("exec", matches)) => {
            let commands = matches.values_of("commands").unwrap().collect::<Vec<&str>>();
            if confirm(format!("Really run '{}' in {}?", commands.join(" "), container_id))? {
                exec(matches, container).await
            } else {
                Ok(())
            }
        },
        Some(("inspect", _)) => inspect(container).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn top(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    let top = container.top(None).await?;
    let hdr = crate::commands::util::mk_header(top.titles.iter().map(|s| s.as_ref()).collect());
    crate::commands::util::display_data(hdr, top.processes, matches.is_present("csv"))
}

async fn kill(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    let signal = matches.value_of("signal");
    container.kill(signal).await.map_err(Error::from)
}

async fn delete(container: Container<'_>) -> Result<()> {
    container.delete().await.map_err(Error::from)
}

async fn start(container: Container<'_>) -> Result<()> {
    container.start().await.map_err(Error::from)
}

async fn stop(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    container.stop({
        matches
            .value_of("timeout")
            .map(u64::from_str)
            .transpose()?
            .map(std::time::Duration::from_secs)
    })
    .await
    .map_err(Error::from)
}

async fn exec(matches: &ArgMatches, container: Container<'_>) -> Result<()> {
    use std::io::Write;
    use futures::TryStreamExt;

    let execopts = shiplift::builder::ExecContainerOptions::builder()
        .cmd({
            matches.values_of("commands").unwrap().collect::<Vec<&str>>()
        })
        .attach_stdout(true)
        .attach_stderr(true)
        .build();

    container.exec(&execopts)
        .map_err(Error::from)
        .try_for_each(|chunk| async {
            match chunk {
                shiplift::tty::TtyChunk::StdIn(_) => Err(anyhow!("Cannot handle STDIN TTY chunk")),
                shiplift::tty::TtyChunk::StdOut(v) => {
                    std::io::stdout().write(&v).map_err(Error::from).map(|_| ())
                },
                shiplift::tty::TtyChunk::StdErr(v) => {
                    std::io::stderr().write(&v).map_err(Error::from).map(|_| ())
                },
            }
        })
        .await
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
    use std::io::Write;
    use itertools::Itertools;

    let d = container.inspect().await?;

    fn option_vec(ov: Option<&Vec<String>>) -> String {
        ov.map(|v| format!("Some({})", v.iter().join(", "))).unwrap_or_else(|| String::from("None"))
    }

    fn option_vec_nl(ov: Option<&Vec<String>>, ind: usize) -> String {
        ov.map(|v| v.iter().map(|s| format!("{:ind$}{s}", "", ind = ind, s = s)).join("\n")).map(|s| format!("\n{}", s)).unwrap_or_else(|| String::from("None"))
    }

    fn option_tostr<T: ToString>(ots: Option<T>) -> String {
        ots.map(|s| format!("Some({})", s.to_string())).unwrap_or_else(|| String::from("None"))
    }

    writeln!(std::io::stdout(), "{}", indoc::formatdoc!(r#"
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

    app_armor_profile = d.app_armor_profile,
    args = d.args.iter().join(", "),

    config_attach_stderr = d.config.attach_stderr.to_string(),
    config_attach_stdin = d.config.attach_stdin.to_string(),
    config_attach_stdout = d.config.attach_stdout.to_string(),
    config_cmd = option_vec(d.config.cmd.as_ref()),
    config_domainname = d.config.domainname,
    config_entrypoint = option_vec(d.config.entrypoint.as_ref()),
    config_env = option_vec_nl(d.config.env.as_ref(), 8),
    config_exposed_ports = {
        d.config.exposed_ports.map(|hm| {
            let s = hm.iter()
                .map(|(k, v_hm)| {
                    format!("{:ind$}{k}:\n{hm}",
                        "", ind = 8,
                        k = k,
                        hm = v_hm.iter()
                            .map(|(k, v)| format!("{:ind$}{k}: {v}", "", ind = 12, k = k, v = v))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!("\n{s}", s = s)
        })
        .unwrap_or_else(|| String::from("None"))
    },
    config_hostname = d.config.hostname,
    config_image = d.config.image,
    config_labels = {
        d.config.labels
            .map(|hm| {
                let s = hm.iter()
                    .map(|(k, v)| format!("{:ind$}{k}: {v}", "", ind = 8, k = k, v = v))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n{s}", s = s)
            })
            .unwrap_or_else(|| String::from("None"))
    },
    config_on_build = option_vec(d.config.on_build.as_ref()),
    config_open_stdin = d.config.open_stdin.to_string(),
    config_stdin_once = d.config.stdin_once.to_string(),
    config_tty = d.config.tty.to_string(),
    config_user = d.config.user,
    config_working_dir = d.config.working_dir,

    created = d.created.to_string(),
    driver = d.driver,

    host_config_cgroup_parent = option_tostr(d.host_config.cgroup_parent.as_ref()),
    host_config_container_id_file = d.host_config.container_id_file,
    host_config_cpu_shares = option_tostr(d.host_config.cpu_shares.as_ref()),
    host_config_cpuset_cpus = option_tostr(d.host_config.cpuset_cpus.as_ref()),
    host_config_memory = option_tostr(d.host_config.memory.as_ref()),
    host_config_memory_swap = option_tostr(d.host_config.memory_swap.as_ref()),
    host_config_network_mode = d.host_config.network_mode,
    host_config_pid_mode = option_tostr(d.host_config.pid_mode.as_ref()),
    host_config_port_bindings = {
        d.host_config.port_bindings
            .map(|hm| {
                let s = hm.iter()
                    .map(|(k, v)| {
                        let v = v.iter()
                            .map(|hm| {
                                hm.iter()
                                    .map(|(k, v)| {
                                        format!("{:ind$}{k}: {v}", "", ind = 12, k = k, v = v)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        format!("{:ind$}{k}: {v}", "", ind = 8, k = k, v = format!("\n{}", v))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n{s}", s = s)
            })
            .unwrap_or_else(|| String::from("None"))
    },
    host_config_privileged = d.host_config.privileged.to_string(),
    host_config_publish_all_ports = d.host_config.publish_all_ports.to_string(),
    host_config_readonly_rootfs = option_tostr(d.host_config.readonly_rootfs.as_ref()),

    hostname_path = d.hostname_path,
    hosts_path = d.hosts_path,
    log_path = d.log_path,
    id = d.id,
    image = d.image,
    mount_label = d.mount_label,
    name = d.name,

    network_settings_bridge = d.network_settings.bridge,
    network_settings_gateway = d.network_settings.gateway,
    network_settings_ip_address = d.network_settings.ip_address,
    network_settings_ip_prefix_len = d.network_settings.ip_prefix_len.to_string(),
    network_settings_mac_address = d.network_settings.mac_address,
    network_settings_ports = {
        d.network_settings.ports 
            .map(|hm| {
                let s = hm.iter()
                    .map(|(k, v)| {
                        let v = v.as_ref().map(|v| {
                            v.iter()
                                .map(|hm| {
                                    let s = hm.iter()
                                        .map(|(k, v)| format!("{:ind$}{k}: {v}", "", ind = 12, k = k, v = v))
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    format!("\n{s}", s = s)
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        }).unwrap_or_else(|| String::from("None"));

                        format!("{:ind$}{k}: {v}", "", ind = 8, k = k, v = format!("\n{}", v))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n{s}", s = s)
            })
            .unwrap_or_else(|| String::from("None"))
    },
    network_settings_networks = {
        let s = d.network_settings.networks.iter().map(|(k, v)| {
            indoc::formatdoc!(r#"
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
            network_id = v.network_id,
            endpoint_id = v.endpoint_id,
            gateway = v.gateway,
            ip_address = v.ip_address,
            ip_prefix_len = v.ip_prefix_len,
            ipv6_gateway = v.ipv6_gateway,
            global_ipv6_address = v.global_ipv6_address,
            global_ipv6_prefix_len = v.global_ipv6_prefix_len.to_string(),
            mac_address = v.mac_address,
            )
            .lines()
            .map(|s| format!("{:ind$}{s}", "", ind = 8, s = s))
            .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n");

        format!("\n{}", s)
    },

    path = d.path,
    process_label = d.process_label,
    resolv_conf_path = d.resolv_conf_path,
    restart_count = d.restart_count.to_string(),

    state_error = d.state.error,
    state_exit_code = d.state.exit_code.to_string(),
    state_finished_at = d.state.finished_at.to_string(),
    state_oom_killed = d.state.oom_killed.to_string(),
    state_paused = d.state.paused.to_string(),
    state_pid = d.state.pid.to_string(),
    state_restarting = d.state.restarting.to_string(),
    state_running = d.state.running.to_string(),
    state_started_at = d.state.started_at.to_string(),
    state_status = d.state.status,

    mounts = {
        let s = d.mounts.iter()
            .map(|mount| {
                indoc::formatdoc!(r#"
                    source: {source}
                    destination: {destination}
                    mode: {mode}
                    rw: {rw}

                "#,
                source = mount.source,
                destination = mount.destination,
                mode = mount.mode,
                rw = mount.rw.to_string()
                )
                .lines()
                .map(|s| format!("{:ind$}{s}", "", ind = 4, s = s))
                .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("\n{}", s)
    }
    )).map_err(Error::from)
}

