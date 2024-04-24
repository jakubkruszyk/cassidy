import sys
import matplotlib.pyplot as plt


def read_user(path):
    with open(path) as file:
        data = file.read()
        data = [int(sample) / 1000 for sample in data.split(",") if sample.strip()]
        return data


def read_station(path):
    results = {}
    with open(path) as file:
        for line in file.readlines():
            data = [int(sample) for sample in line.split(",") if sample.strip()]
            results[data[0]] = data[1:]
    return results


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(
            "Not enough arguments: usage python3 rng_histograms.py station.log user.log"
        )
        quit()
    station_path = sys.argv[1]
    user_path = sys.argv[2]

    station_data = read_station(station_path)
    user_data = read_user(user_path)

    plt.hist(user_data, bins=20)
    plt.show()
