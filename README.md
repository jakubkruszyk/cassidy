# Cassidy

Simulation tool for radiocommunication basestations system.
Written as university project.

## Specification

Let us consider a radiocommunication system consisting of N (5) basestations with R (273) resource blocks.
At random intervals of time 𝜏 (resulting from exponential distribution) users appear at each basestation.
Each user occupies single resource block for a random time μ from range <1, 15> seconds. If the basestation does 
not have enough resource blocks to handle the user, request may be redirected to a other station. If neither of
basestations can handle the request, user is considered lost. Intensity of reports in the system varies cyclically:

- for the first 8 hours, the intensity of reports is λ/2
- next it is 3λ/4 for 6 hours, 
- then it is λ for 4 hours,
- then decreases to 3λ/4 for 6 hours

Next the cycle repeats itself.
Each basestation has a threshold L (expressed in % of resource blocks used) for entering the sleep state.
The basestation in the sleep state consumes power equal to 1 W, and 200W when it is active. Users from the station in
sleep state are distributed evenly to the other stations. Similarly, if the threshold H (expressed in % of resource blocks occupied)
is exceeded in one of the stations, the station in sleep state is activated. Sleep and activation processes take 50 ms 
and consumes 1000 W at a time.

## Usage

```shell
cassidy [Options] --duration <f64>
```

### Options
| Option | Description|
|--------|------------|
| --with-config <path> | Path to simulation config file |
| --seed <u64> | Seed for random number generator |
| --log | Generate event log file |
| --duration <time> | Time (in hours) simulation will be run for. Maximum precision is 1ms |
| --iterations <u32> | Simulation iterations count. Default value is 1 |
| --enable-sleep | Enable sleep state logic |
| --save-default-config <path> | Save default config |
| --show-partial-results | Show partial results from all iterations |
| --log-wave | Log simulation process in binary format |
| --samples <u32> | Binary log sampling divider [default: 1] |
| --walk-over <path> | Enable iteration over given parameter based on given config |
| -h, --help | Print help |
| -V, --version | Print version |

### Examples

Run single iteration with default configuration for 24 hours simulation time 

```shell
cassidy --duration 24
```

Run 10 iterations using my_cfg.toml config file for 24 hours simulation time

```shell
cassidy --duration 24 --iterations 10 --with-config my_cfg.toml
```

Run 10 iterations using my_cfg.toml config file for 24 hours simulation time with sleep mode and partial results printing enabled

```shell
cassidy --duration 24 --iterations 10 --with-config my_cfg.toml --enable-sleep --show-partial-results
```

Run 1 iteration using my_cfg.toml config file for 24 hours simulation time for each parameter value specified in my_walk_cfg.toml config file

```shell
cassidy --duration 24 --iterations 10 --with-config my_cfg.toml --walk-over my_walk_cfg.toml
```

## Configuration files

### Simulation configuration

Configuration file that specifies every simulation parameters except total duration. Below is default configuration that can be also obtained with `--save-default-config` option:

```toml
process_time_max = 15000 # Upper range for user's processing time
process_time_min = 1000  # Lower range for user's processing time. Each user has random processing time from range <lower, upper>, endpoints included 
lambda = 10.0            # Average number of arriving users per second. This value will be multiplied by lambda coefficient from lambda_coefs list
resources_count = 273    # Number of resource blocks in each station
sleep_threshold = 20     # Threshold from range <0, 100>%. If station usage is below this threshold, station will try to change mode to sleep 
wakeup_threshold = 80    # Threshold from range <0, 100>%. If station usage is above this threshold, system will try to wake up single station in sleep mode
stations_count = 10      # Number of stations
active_power = 200.0     # Power usage when station is active
sleep_power = 1.0        # Power usage when station is in sleep mode
wakeup_power = 1000.0    # Singular power usage when station is transitioning from sleep mode to active and vice versa
wakeup_delay = 50        # Delay between changing mode from sleep to active and vice versa
log_buffer = 10000       # Size of logger internal buffer. Currently does not matter

[[lambda_coefs]]         # List of pairs (lambda_coefficient, duration)
time = 8.0               # Duration, in hours, of this phase
coef = 0.5               # Lambda coefficient for this phase

[[lambda_coefs]]
time = 6.0               # Duration is relative, so this phase is active for 6 hours after previus phase. In this example from time = 8h to time = 14h
coef = 0.75

[[lambda_coefs]]
time = 4.0               # If total simulation duration is longer than sum of all phases, cycle will start again from beginning
coef = 1.0

[[lambda_coefs]]
time = 6.0
coef = 0.75
```

### Walk-over configuration

Configuration file for walk-over mode. There is no default configuration, so user must provide it's own. All configuration fields are shown below:

```toml
var = "Lambda"  # Name of variable that can be iterated over. Possible values are Lambda, SleepLow (sleep_threshold) and SleepHigh (wakeup_threshold)
start = 10.0    # Start value of the variable
end = 50.0      # End value of the variable (included)
step = 5.0      # Value that will be added to parameter after each iteration. In this example simulation will be run for lambda values: [10, 15, 20, 25, 30, 35, 40, 45, 50]
```

## Logs
Cassid can produce one or more of four log types:
- when `walk-over` option is NOT specified results in human readable form will be printed and saved to sim_report file
- when `walk-over` option is specified results in CSV format are saved to sim_report file. First line contains names of saved parameters and first column always contain values of parameter specified in walk-over config file
- when `--log` option is specified every processed event will be written to separate log file under "sim.run_[run_no]_no_[iteration_no]".
- when `--log-wave` option is specified usage and state of station will be written in binary format every processed event to separate log file under "sim_bin.run_[run_no]_no_[iteration_no]"

## Utility scripts
For convenience, in `scripts/` directory, there are python scripts for viewing simulation results.

### parse_bin_log.py
Script for viewing binary logs generated from `--log-wave` option. 

`Usage: python parse_bin_log.py <path to log> [subsampling]`

Example:

`python parse_bin_log.py sim.log 100`

Script will read every 100th sample from sim.log file. By default only data from first station will be displayed. User can show/ hide the data from other stations by clicking on appropriate line on plot legend.


### parse_csv_log.py
Script for viewing data from csv report generated with `--walk-over` option.

`Usage: python parse_csv_log.py <path to log>`

### rng_histograms.py
Script for viewing results from random number generators used for user's processing time and next user arrival timestamp in stations. To generate files with rngs data run command below:

`cargo test -- test_rng generate_lambda`

To view the results run command below from `script/` directory:

`python rng_histograms.py`
