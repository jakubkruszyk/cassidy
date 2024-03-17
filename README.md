# Cassidy
Simulation tool for radiocommunication basestations system.
Written as university project.

## Specification
Let us consider a radiocommunication system consisting of N (5) basestations with R (273) resource blocks.
At random intervals of time 𝜏 (resulting from exponential distribution) users appear at each basestation.
Each user occupies single resource block for a random time μ from range <1, 15> seconds. If the basestation does 
not have enough resource blocks to handle the user, request may be redirected to a other station. If neither
basestations cannot handle the request, user is considered lost. Intensity of reports in the system varies cyclically:
- for the first 8 hours, the intensity of reports is λ/2
- next it is 3λ/4 for 6 hours, 
- then it is λ for 4 hours,
- then decreases to 3λ/4 for 6 hours

Next the cycle repeats itself.
Each basestation has a threshold L (expressed in % of resource blocks used) for entering the sleep state.
The basestation in the sleep state consumes power equal to 1 W, and 200W when it is active. Reports from the station in
sleep state are distributed evenly to the other stations. Similarly, if the threshold H (expressed in % of resource blocks occupied)
is exceeded in one of the stations, the station in sleep state is activated. Sleep and activation processes take 50 ms 
and consumes 1000 W at a time.
