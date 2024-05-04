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
    station_path = "../tests/station_rng.log"
    user_path = "../tests/user_rng.log"

    station_data = read_station(station_path)
    user_data = read_user(user_path)

    station1 = list(station_data.values())[0]

    fig, (ax1, ax2) = plt.subplots(1, 2)
    ax1.hist(user_data, bins=20)
    ax1.set_title("User RNG")
    ax2.hist(station1, bins=20)
    ax2.set_title("Station RNG")

    plt.show()
