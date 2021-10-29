import yaml
import pyron
import click

# Converts YAML files to Ron files.
@click.command()
@click.argument("files", nargs=-1)
def main(files):
    for file in files:
        with open(file) as f:
            data = yaml.full_load(f)
        # Replace file extension with .ron
        ron_file = file.replace(".yaml", ".ron")
        # Ensure new file has a different name
        if ron_file == file:
            ron_file = file + ".ron"
        print(data)
        serialized = pyron.to_string(data)
        with open(ron_file, "w") as f:
            f.write(serialized)


if __name__ == "__main__":
    main()
