import os
import sys
import time
import urllib.error
import urllib.request


DEFAULT_RETRIES = 3
DEFAULT_TIMEOUT_SECONDS = 60
CHUNK_SIZE = 1024 * 1024


def download_file(url, output_path, output_filename):
    os.makedirs(output_path, exist_ok=True)

    local_filename = os.path.join(output_path, output_filename)
    part_filename = local_filename + ".part"
    print("Downloading file from {} to {}".format(url, local_filename))

    last_error = None
    for attempt in range(1, DEFAULT_RETRIES + 1):
        try:
            if os.path.exists(part_filename):
                os.remove(part_filename)

            with urllib.request.urlopen(url, timeout=DEFAULT_TIMEOUT_SECONDS) as response:
                total_length = response.getheader("content-length")
                expected_length = int(total_length) if total_length is not None else None
                downloaded = 0

                with open(part_filename, "wb") as f:
                    while True:
                        chunk = response.read(CHUNK_SIZE)
                        if not chunk:
                            break
                        f.write(chunk)
                        downloaded += len(chunk)
                        if expected_length:
                            done = int(50 * downloaded / expected_length)
                            sys.stdout.write(
                                "\r[{}{}] {:.2f}%".format(
                                    "=" * done,
                                    " " * (50 - done),
                                    (downloaded / expected_length) * 100,
                                )
                            )
                            sys.stdout.flush()
                    f.flush()
                    os.fsync(f.fileno())

            if expected_length is not None and downloaded != expected_length:
                raise RuntimeError(
                    "downloaded {} bytes, expected {} bytes".format(
                        downloaded, expected_length
                    )
                )

            os.replace(part_filename, local_filename)
            print("\nDownload completed!")
            return
        except (OSError, RuntimeError, urllib.error.URLError) as error:
            last_error = error
            if os.path.exists(part_filename):
                os.remove(part_filename)
            if attempt < DEFAULT_RETRIES:
                delay = attempt * 2
                print(
                    "\nDownload attempt {}/{} failed: {}. Retrying in {}s...".format(
                        attempt, DEFAULT_RETRIES, error, delay
                    ),
                    file=sys.stderr,
                )
                time.sleep(delay)

    raise RuntimeError(
        "failed to download {} after {} attempts: {}".format(
            url, DEFAULT_RETRIES, last_error
        )
    )


if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python download_file.py <URL> <output_path> <output_filename>")
        sys.exit(1)

    try:
        download_file(sys.argv[1], sys.argv[2], sys.argv[3])
    except Exception as error:
        print("Download failed: {}".format(error), file=sys.stderr)
        sys.exit(1)
