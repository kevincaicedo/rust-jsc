import os
import sys
import urllib.request

def download_file(url, output_path, output_filename):
    if not os.path.exists(output_path):
        os.makedirs(output_path)

    local_filename = os.path.join(output_path, output_filename)
    
    with urllib.request.urlopen(url) as response:
        total_length = response.getheader('content-length')
        
        if total_length is None:
            print("Content length not provided by server, cannot show progress.")
            with open(local_filename, 'wb') as f:
                f.write(response.read())
        else:
            total_length = int(total_length)
            downloaded = 0
            chunk_size = 8192
            
            with open(local_filename, 'wb') as f:
                while True:
                    chunk = response.read(chunk_size)
                    if not chunk:
                        break
                    f.write(chunk)
                    downloaded += len(chunk)
                    done = int(50 * downloaded / total_length)
                    sys.stdout.write("\r[{}{}] {:.2f}%".format('=' * done, ' ' * (50-done), (downloaded / total_length) * 100))
                    sys.stdout.flush()
    
    print("\nDownload completed!")

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python script.py <URL> <output_path> <output_filename>")
        sys.exit(1)
    
    url = sys.argv[1]
    output_path = sys.argv[2]
    output_filename = sys.argv[3]
    
    download_file(url, output_path, output_filename)