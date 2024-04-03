def parse(path, subsamples=100):
    file = open(path, "rb")
    stations = file.read(4)
    if len(stations) < 4:
        raise ValueError("Couldn't parse stations count")
    stations = int.from_bytes(stations, byteorder="little")
    print(f"stations: {stations}")
    chunk_size = 8 + stations * 5
    timestamps = list()
    stations_usage = [list() for _ in range(stations)]
    stations_state = [list() for _ in range(stations)]
    temp = False
    while True:
        # timestamp: 8 bytes
        # each station:
        # - usage: 4 bytes
        # - state: 1 byte
        chunk = file.read(chunk_size)
        if len(chunk) < chunk_size:
            break

        timestamps.append(int.from_bytes(chunk[0:8], byteorder="little"))
        for i in range(stations):
            stations_usage[i].append(
                int.from_bytes(chunk[8 + i * 5 : 12 + i * 5], byteorder="little")
            )
            stations_state[i].append(chunk[12 + i * 5])

        print(file.tell())
        if temp:
            break
        else:
            temp = True
        next_sample = file.tell() + subsamples * chunk_size
        file.seek(next_sample)

    file.close()
    return timestamps, stations_usage, stations_state


if __name__ == "__main__":
    res = parse("../wave.log")
    print(len(res[0]))
