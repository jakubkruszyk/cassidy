import matplotlib.pyplot as plt
import sys


def read_report(path):
    with open(path) as file:
        lines = file.readlines()
        tags = lines[0].strip().split(",")
        tag_no = len(tags)
        data = [list() for _ in range(tag_no)]
        for line in lines[1:]:
            for i, value in enumerate(line.strip().split(",")):
                data[i].append(float(value))
        variable = (tags.pop(0), data.pop(0))
        data_combined = {}
        for tag, data_list in zip(tags, data):
            data_combined[tag] = data_list

    return variable, data_combined


if __name__ == "__main__":
    # Parse cmd arguments
    # python3 parse_csv_report.py <path_to_report>
    if len(sys.argv) < 2:
        print(
            "Not enough arguments: usage python3 parse_csv_report.py <path_to_report>"
        )
        quit()

    path = sys.argv[1]
    var, data = read_report(path)

    # Get from user which lines to plot
    keys = list(data.keys())
    for i, tag in enumerate(keys):
        print(f"{i}: {tag}")

    print("Select lines to plot. Numbers can be separated by spaces:")
    numbers_str = input()
    numbers = [int(x) for x in numbers_str.split(" ")]
    lines_to_plot = [keys[i] for i in numbers]

    # Create plot
    fig, ax = plt.subplots()
    ax.grid(True)
    ax.set_xlabel(var[0])
    for data_name in lines_to_plot:
        (line,) = ax.plot(var[1], data[data_name], label=data_name)

    # Legend
    leg = ax.legend(fancybox=True, shadow=True)
    leg.set_draggable(True)
    plt.show()
