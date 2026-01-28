import os
import sys
import subprocess
import argparse

def is_text_file(file_path):
    """
    Check if a file is a text file by reading a small chunk and looking for null bytes.
    Also skips known large binary extensions.
    """
    # Quick extension check
    binary_extensions = {
        '.png', '.jpg', '.jpeg', '.gif', '.bmp', '.ico', '.tiff', '.webp',
        '.mp3', '.wav', '.ogg', '.flac', '.aac', '.m4a',
        '.mp4', '.mov', '.avi', '.mkv', '.webm',
        '.zip', '.tar', '.gz', '.7z', '.rar',
        '.exe', '.dll', '.so', '.dylib', '.bin', '.obj', '.o', '.a', '.lib',
        '.pdf', '.doc', '.docx', '.xls', '.xlsx', '.ppt', '.pptx',
        '.pyc', '.pyo', '.pyd', '.class', '.jar', '.war', '.ear',
        '.db', '.sqlite', '.sqlite3', '.nc', '.nicnt', '.nkx', '.nki', '.nkm', '.nkr'
    }
    
    _, ext = os.path.splitext(file_path)
    if ext.lower() in binary_extensions:
        return False

    try:
        with open(file_path, 'rb') as f:
            chunk = f.read(1024)
            if b'\x00' in chunk:
                return False
            # Check if it looks like UTF-8 (or ASCII)
            try:
                chunk.decode('utf-8')
            except UnicodeDecodeError:
                # If it's not valid UTF-8, it might be some other encoding, 
                # but for code it's usually safest to skip or treat as binary if uncertain.
                # Let's try latin-1 as a fallback for some legacy comments, but generally 
                # null bytes are the best indicator for binary.
                pass
    except Exception:
        return False
        
    return True

def get_git_files(repo_path):
    """
    Get a list of files tracked by git in the given repo_path using 'git ls-files'.
    Returns a list of relative paths from repo_path, or None if git fails.
    """
    try:
        # Check if it's a git repo
        if not os.path.isdir(os.path.join(repo_path, '.git')):
             # It might be a subdirectory of a git repo
             pass

        result = subprocess.run(
            ['git', 'ls-files', '--cached', '--others', '--exclude-standard'],
            cwd=repo_path,
            capture_output=True,
            text=True,
            check=True
        )
        files = result.stdout.strip().splitlines()
        # Filter out empty strings if any
        return [f for f in files if f.strip()]
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None

def main():
    parser = argparse.ArgumentParser(description="Merge code files into a single context file for LLM analysis.")
    parser.add_argument("--source", "-s", default=r"..", help=r"Source directory to scan (default: ..)")
    parser.add_argument("--output", "-o", default=r"..\MonitorControllerMax_CodeContext.txt", help="Output file name (default: ..\MonitorControllerMax_CodeContext.txt)")
    
    args = parser.parse_args()
    
    # Resolve absolute paths
    script_dir = os.path.dirname(os.path.abspath(__file__))
    source_dir = os.path.abspath(os.path.join(script_dir, args.source))
    
    # If output is just a filename, put it in the script directory (as requested)
    if os.path.dirname(args.output):
        output_file = os.path.abspath(args.output)
    else:
        output_file = os.path.join(script_dir, args.output)

    print(f"Scanning source: {source_dir}")
    print(f"Output target:   {output_file}")

    if not os.path.isdir(source_dir):
        print(f"Error: Source directory '{source_dir}' does not exist.")
        sys.exit(1)

    file_list = []
    
    # 1. Try git ls-files first
    print("Attempting to use git to list files...")
    git_files = get_git_files(source_dir)
    
    if git_files is not None:
        print(f"Found {len(git_files)} files using git.")
        # Git returns paths relative to source_dir
        for rel_path in git_files:
            abs_path = os.path.join(source_dir, rel_path)
            if os.path.isfile(abs_path):
                file_list.append((rel_path, abs_path))
    else:
        print("Git method failed or not a git repo. Falling back to manual walk.")
        # 2. Fallback: Manual walk
        exclude_dirs = {'.git', '.svn', '.hg', 'target', 'build', 'bin', 'obj', 'node_modules', '.idea', '.vscode'}
        
        for root, dirs, files in os.walk(source_dir):
            # Modify dirs in-place to skip ignored directories
            dirs[:] = [d for d in dirs if d not in exclude_dirs]
            
            for file in files:
                abs_path = os.path.join(root, file)
                rel_path = os.path.relpath(abs_path, source_dir)
                file_list.append((rel_path, abs_path))

    # Sort files for consistent output
    file_list.sort(key=lambda x: x[0])

    print(f"Processing {len(file_list)} candidate files...")
    
    count = 0
    skipped_binary = 0
    skipped_lock = 0
    
    with open(output_file, 'w', encoding='utf-8') as outfile:
        # Write a header
        outfile.write(f"Context generated from: {source_dir}\n")
        outfile.write(f"File count: {len(file_list)}\n")
        outfile.write("-" * 80 + "\n")
        
        # Write Table of Contents
        outfile.write("Table of Contents:\n")
        index = 1
        for rel_path, abs_path in file_list:
            if rel_path.endswith('Cargo.lock') or rel_path.endswith('package-lock.json') or rel_path.endswith('yarn.lock'):
                continue
            if not is_text_file(abs_path):
                continue
            outfile.write(f"{index}. {rel_path}\n")
            index += 1
        outfile.write("-" * 80 + "\n\n")

        for rel_path, abs_path in file_list:
            # Skip lock files as they are usually too verbose and not useful logic
            if rel_path.endswith('Cargo.lock') or rel_path.endswith('package-lock.json') or rel_path.endswith('yarn.lock'):
                skipped_lock += 1
                continue
                
            if not is_text_file(abs_path):
                skipped_binary += 1
                continue

            try:
                with open(abs_path, 'r', encoding='utf-8', errors='ignore') as infile:
                    content = infile.read()
                    
                    separator = "=" * 80
                    outfile.write(f"{separator}\n")
                    outfile.write(f"File Path: {rel_path}\n")
                    outfile.write(f"{separator}\n")
                    outfile.write(content)
                    outfile.write("\n\n")
                    count += 1
            except Exception as e:
                print(f"Error reading {rel_path}: {e}")

    print(f"Done.")
    print(f"  Processed: {count} files")
    print(f"  Skipped (Binary): {skipped_binary} files")
    print(f"  Skipped (Lockfiles): {skipped_lock} files")
    print(f"Generated: {output_file}")

if __name__ == "__main__":
    main()
