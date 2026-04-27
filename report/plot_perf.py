import matplotlib.pyplot as plt

# Load your data
data = []
with open('perf.txt', 'r') as f:
    for line in f:
        # Clean potential citation tags and split into numbers
        parts = line.replace('', '').split()
        for part in parts:
            try:
                data.append(float(part))
            except ValueError:
                continue

# Generate the plot
plt.figure()
plt.bar(range(len(data)), data, width=1.0)

# Reference lines for common frame rates
plt.axhline(y=16.67, color='r', linestyle='--', label='60 FPS (16.67ms)')
plt.axhline(y=33.33, color='orange', linestyle='--', label='30 FPS (33.33ms)')

plt.xlabel('Frame Number')
plt.ylabel('Time (ms)')
plt.title('Frame processing time')
plt.legend()
plt.grid(axis='y', alpha=0.3)
plt.savefig('performance_plot.png')
