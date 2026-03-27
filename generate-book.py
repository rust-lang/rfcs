#!/usr/bin/env python3

"""
This auto-generates the mdBook SUMMARY.md file based on the layout on the filesystem.

This generates the `src` directory based on the contents of the `text` directory.

Most RFCs should be kept to a single chapter. However, in some rare cases it
may be necessary to spread across multiple pages. In that case, place them in
a subdirectory with the same name as the RFC. For example:

    0123-my-awesome-feature.md
    0123-my-awesome-feature/extra-material.md

It is recommended that if you have static content like images that you use a similar layout:

    0123-my-awesome-feature.md
    0123-my-awesome-feature/diagram.svg

The chapters are presented in sorted-order.
"""

import os
import shutil
import subprocess

def main():
    if os.path.exists('src'):
        # Clear out src to remove stale links in case you switch branches.
        shutil.rmtree('src')
    os.mkdir('src')

    for path in os.listdir('text'):
        symlink(f'../text/{path}', f'src/{path}')
    symlink(f'../compiler_changes.md', f'src/compiler_changes.md')
    symlink(f'../lang_changes.md', f'src/lang_changes.md')
    symlink(f'../libs_changes.md', f'src/libs_changes.md')
    symlink('../README.md', 'src/introduction.md')

    with open('src/SUMMARY.md', 'w') as summary:
        summary.write('[Introduction](introduction.md)\n\n')
        summary.write('- [Guidelines for compiler changes](compiler_changes.md)\n')
        summary.write('- [Guidelines for language changes](lang_changes.md)\n')
        summary.write('- [Guidelines for library changes](libs_changes.md)\n')
        collect(summary, 'text', 0)

    subprocess.call(['mdbook', 'build'])

def collect(summary, path, depth):
    entries = [e for e in os.scandir(path) if e.name.endswith('.md')]
    entries.sort(key=lambda e: e.name)
    for entry in entries:
        indent = '    '*depth
        name = entry.name[:-3]
        link_path = entry.path[5:]
        summary.write(f'{indent}- [{name}]({link_path})\n')
        maybe_subdir = os.path.join(path, name)
        if os.path.isdir(maybe_subdir):
            collect(summary, maybe_subdir, depth+1)

def symlink(src, dst):
    if not os.path.exists(dst):
        os.symlink(src, dst)

if __name__ == '__main__':
    main()
