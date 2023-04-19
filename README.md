# x3d-pinner
## Goal
Automatically execute `taskset` for certain processes (i.e. games) to execute on the Zen 4's V-Cache CCD only.

## Implementation
Very basic daemon that automatically executes configured commands if the start of the command line text (`/proc/*/cmd`) matches with a configured process name in the configuration file for a process owned by a configured username. Commands are only executed once for every PID.

# Configuration
The configuration is read from the file `/etc/x3d-pinner.ini`.

General configuration:

| Key                | Description                                                                                         | Default value |
|--------------------|-----------------------------------------------------------------------------------------------------|---------------|
| username           | Username of user who will be running the commands configured by this daemon                         |               |
| sleep              | Amount of milliseconds to pause between checks for new processes.                                   | 5000          |
| allow-root-process | Add command-line to list of processes owned by root that are allowed to be modified by this daemon. |               |
| exclude-process    | Command-line of processes to never modify.                                                          |               |

Configuration per section:

| Key            | Description                                                           |
|----------------|-----------------------------------------------------------------------|
| [section-name] | Human-readable name for command to be executed.                       |
| command        | Which command to execute. Include placeholder `{}` to insert the PID. |
| process        | **Start** of command-line (`/proc/*/cmd`) to match with.              |

## Example
This example will, for the user `myusername`, automatically pin game `ffxiv_dx11.exe` to the CCD with 3D V-Cache on a 7950X3D, while also pinning `Discord` and `sway` to the high-frequency CPU, while explicitly ignoring `sleep`. It will perform a check for new processes every `5000` milliseconds.
```ini
username=myusername
sleep=5000
allow-root-process=sway
exclude-process=sleep

[vcache]
command=/usr/sbin/taskset -pc 0-7,16-23 {}
process=Z:\\home\\myusername\\.xlcore\\ffxiv\\game\\ffxiv_dx11.exe

[highfreq]
command=/usr/sbin/taskset -pc 8-15,24-31 {}
process=/opt/discord/Discord
process=sway
```

## Example systemd unit
`/usr/lib/systemd/system/x3d-pinner.service`
```
[Unit]
Description=Service for automatically renicing and pinning processes to AMD V-Cache CCD.

[Service]
ExecStart=/usr/local/bin/x3d-pinner

[Install]
WantedBy=multi-user.target
```
