use std::collections::HashMap;
use std::{thread, time};
use std::io::Error;
use std::process::{Command, Output};
use ini::{Ini, Properties};
use sysinfo::{System, Pid, Process, Uid, RefreshKind, ProcessRefreshKind, SystemExt, UserExt, ProcessExt, PidExt};

struct ProcessConfig {
    allowed_root_processes: Vec<String>,
    processes_to_exclude: Vec<String>,
    command_configs: Vec<ProcessCommandConfig>,
    sleep_millis: u64,
}

struct ProcessCommandConfig {
    name: String,
    command: String,
    processes: Vec<String>,
}

fn main() {
    let ini_file = Ini::load_from_file("/etc/x3d-pinner.ini");
    if ini_file.is_err() {
        panic!("Failed to load configuration file /etc/x3d-pinner.ini. {}", ini_file.err().unwrap())
    }
    let conf = Ini::load_from_file("/etc/x3d-pinner.ini").unwrap();
    let username = conf.general_section().get("username");
    if username.is_none() {
        panic!("No username configured");
    }
    let username_value = username.unwrap();

    let mut s = System::new_with_specifics(RefreshKind::new().with_users_list());
    s.refresh_users_list();

    let user = s.users().iter().find(|&u| u.name() == username_value);
    if user.is_none() {
        panic!("No user named {}", username_value);
    }
    let user_uid = user.unwrap().id();

    let process_config_value: ProcessConfig = ProcessConfig {
        allowed_root_processes: get_config_value(conf.general_section(), "allow-root-process"),
        processes_to_exclude: get_config_value(conf.general_section(), "exclude-process"),
        command_configs: load_command_configs(&conf),
        sleep_millis: conf.general_section().get("sleep").iter().flat_map(|&s| s.parse::<u64>()).next().unwrap_or(5000),
    };
    println!("Sleep configured for {} millis", process_config_value.sleep_millis);
    println!("Loaded {} allowed-root-processes", process_config_value.allowed_root_processes.iter().count());
    println!("Loaded {} processes_to_exclude", process_config_value.processes_to_exclude.iter().count());
    println!("Loaded {} command_configs", process_config_value.command_configs.iter().count());

    println!("Started x3d-pinner");
    start(user_uid, process_config_value);
}

fn load_command_configs(conf: &Ini) -> Vec<ProcessCommandConfig> {
    let mut configs: Vec<ProcessCommandConfig> = Vec::new();
    for section in conf.sections().filter(|s| s.is_some()).map(|s| s.unwrap()).filter(|&s| s != "") {
        let section_properties_option = conf.section(Some(section));
        if section_properties_option.is_some() {
            let section_properties = section_properties_option.unwrap();
            let command = section_properties.get("command");
            if command.is_none() {
                panic!("Section {} has no command", section);
            }
            let processes = get_config_value(section_properties, "process");
            let process_command_config = ProcessCommandConfig {
                name: section.to_string(),
                command: command.unwrap().to_string(),
                processes,
            };
            println!("Loaded {} with command {} with {} processes", process_command_config.name, process_command_config.command, process_command_config.processes.iter().count());
            configs.push(process_command_config);
        }
    }
    if configs.is_empty() {
        panic!("No commands configured.");
    }
    return configs;
}

fn get_config_value(properties: &Properties, key: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for value in properties.get_all(key) {
        result.push(value.to_string());
    }
    return result
}

fn start(user_uid: &Uid, process_config: ProcessConfig) {
    let mut s = System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new().with_user()));
    let mut already_processed: Vec<u32> = Vec::new();
    loop {
        s.refresh_processes();
        let process_list = s.processes();

        let to_process: HashMap<&Pid, &Process> = process_list.iter()
            .filter(|&(_k, v)| v.user_id().is_some())
            .filter(|&(_k, v)| v.user_id().unwrap() == user_uid || matches(get_process_image_name(v), &process_config.allowed_root_processes)) // only non-root
            .filter(|&(k, _v)| !already_processed.contains(&k.as_u32()))
            .filter(|&(_k, v)| !matches(get_process_image_name(v), &process_config.processes_to_exclude))
            .collect();

        for (pid, process) in to_process {
            execute(pid, process, &process_config);
        }

        already_processed.clear();
        already_processed.extend(process_list.iter().map(|(k, _v)| k.as_u32()));
        thread::sleep(time::Duration::from_millis(process_config.sleep_millis));
    }
}

fn execute(pid: &Pid, process: &Process, process_config: &ProcessConfig) {
    let mut match_count: u32 = 0;
    for cmd in &process_config.command_configs {
        if matches(get_process_image_name(process), &cmd.processes) {
            match_count += 1;
            execute_command(pid, process, &cmd);
        }
    }

    if match_count == 0 {
        println!("Implicitly ignoring {} ({})", get_process_image_name(process).as_str(), pid.as_u32())
    }
}

fn matches(process_name: String, data: &Vec<String>) -> bool {
    let expect = process_name.as_str();
    data.iter().find(|&s| expect.starts_with(s)).is_some()
}

fn get_process_image_name(process: &Process) -> String {
    process.cmd().join(" ")
}

fn execute_command(pid: &Pid, process: &Process, process_command_config: &ProcessCommandConfig) {
    let command_string = process_command_config.command.replace("{}", pid.as_u32().to_string().as_str());
    let command_split: Vec<&str> = command_string.split(' ').collect();
    let command_string_rebuilt: String = command_split[0].to_string() + " " + command_split[1..].join(" ").as_str();
    match command_result(Command::new(command_split[0])
        .args(command_split[1..].iter())
        .output()) {
        Ok(v) => println!("Command {} ({}) for process {} (pid {}) result {} {}", process_command_config.name, command_string_rebuilt, process.name(), pid.as_u32(), String::from_utf8(v.stdout).unwrap(), String::from_utf8(v.stderr).unwrap()),
        Err(e) => eprintln!("Command {} ({}) failed for process {} (pid {}) result {}", process_command_config.name, command_string_rebuilt, process.name(), pid.as_u32(), e.to_string()),
    }
}

fn command_result(result: Result<Output, Error>) -> Result<Output, String> {
    return if result.is_ok() {
        let cmd_output = result.unwrap();
        if !cmd_output.status.success() {
            let output: String = "ExitCode is non-zero: ".to_string() + cmd_output.status.code().unwrap().to_string().clone().as_str() + " " + String::from_utf8(cmd_output.stdout).unwrap().as_str() + " " + String::from_utf8(cmd_output.stderr).unwrap().as_str();
            Err(output)
        } else {
            Ok(cmd_output)
        }
    } else {
        Err(result.err().unwrap().to_string().clone())
    }
}
