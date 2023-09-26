# /usr/bin/env python

"""
The above code defines a function called `switch` that takes a list of wallpapers, determines the
current wallpaper, selects the next wallpaper in the list, and sets it as the new wallpaper.
"""


from typing import List
from os import system
from sys import argv, stderr
import subprocess


def switch(wallpapers: List[str]) -> None:
    """
    Takes a list of wallpapers, determines the current wallpaper, finds the index
    of the current wallpaper in the list, selects the next wallpaper in the list,
    and sets it as the new wallpaper.
    """
    if len(wallpapers) == 0:
        print('no wallpapers has been specified', file=stderr)

    result: str = subprocess.check_output(['swww', 'query']).decode('ascii')

    current_wallpaper_path: str = result.split()[7]

    print(current_wallpaper_path)
    print(wallpapers)

    index: int

    try:
        index = wallpapers.index(current_wallpaper_path) + 1
        if index == len(wallpapers):
            index = 0
    except ValueError:
        print('index of current wallpaper not found, using first...', file=stderr)

        index = 0

    next_wallpaper_path = wallpapers[index]

    print(f'next wallpaper path is {next_wallpaper_path}...')

    system(f"swww img {next_wallpaper_path}")


def main():
    """
    The main function takes command line arguments and passes them to the switch function.
    """
    switch(argv[1:])


if __name__ == '__main__':
    main()
