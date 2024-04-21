import sys
import matplotlib.pyplot as plt

state_map = {1: "Sleep", 2: "PowerDown", 3: "PowerUp", 4: "Active"}


def parse(path, subsamples=100):
    file = open(path, "rb")
    stations = file.read(4)
    if len(stations) < 4:
        raise ValueError("Couldn't parse stations count")
    stations = int.from_bytes(stations, byteorder="little")
    print(f"Number of stations: {stations}")
    chunk_size = 8 + stations * 5
    timestamps = list()
    stations_usage = [list() for _ in range(stations)]
    stations_state = [list() for _ in range(stations)]
    while True:
        # timestamp: 8 bytes
        # each station:
        # - usage: 4 bytes
        # - state: 1 byte
        chunk = file.read(chunk_size)
        if len(chunk) < chunk_size:
            break

        time_micro = int.from_bytes(chunk[0:8], byteorder="little")
        timestamps.append(time_micro / (3600 * 1e6))
        for i in range(stations):
            stations_usage[i].append(
                int.from_bytes(chunk[8 + i * 5 : 12 + i * 5], byteorder="little")
            )
            stations_state[i].append(state_map.get(int(chunk[12 + i * 5])))

        next_sample = file.tell() + subsamples * chunk_size
        file.seek(next_sample)

    file.close()
    return timestamps, stations_usage, stations_state


if __name__ == "__main__":
    path = sys.argv[1]
    if len(sys.argv) > 2:
        subsamples = int(sys.argv[2])
    else:
        subsamples = 1
    print(f"Log file path: {path}")
    print(f"Subsampling: {subsamples}")
    res = parse(path, subsamples)
    print(f"Samples: {len(res[0])}")

    # plot results
    fig, (ax_usage, ax_state) = plt.subplots(2)
    ax_usage.grid(True)
    ax_state.grid(True)
    lines = list()
    for i, (usage_list, state_list) in enumerate(zip(res[1], res[2])):
        (line_usage,) = ax_usage.plot(res[0], usage_list, label=f"Station {i}")
        (line_state,) = ax_state.plot(res[0], state_list)
        lines.append((line_usage, line_state))

    # interactive legend
    leg = ax_usage.legend(fancybox=True, shadow=True)
    leg.set_draggable(True)
    pickrad = 5
    leg_lines_map = {}

    for legend_line, plot_line in zip(leg.get_lines(), lines):
        legend_line.set_picker(pickrad)
        leg_lines_map[legend_line] = plot_line

    # Hide all plot lines but first
    for legend_line, plot_line in zip(leg.get_lines()[1:], lines[1:]):
        for line in plot_line:
            line.set_visible(False)
        legend_line.set_alpha(0.2)

    def on_click_callback(event):
        legend_line = event.artist
        if legend_line not in leg_lines_map:
            return

        plot_line = leg_lines_map[legend_line]
        visible = not plot_line[0].get_visible()
        for line in plot_line:
            line.set_visible(visible)
        legend_line.set_alpha(1.0 if visible else 0.2)
        fig.canvas.draw()

    fig.canvas.mpl_connect("pick_event", on_click_callback)
    plt.show()
