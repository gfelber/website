```c
#include <arpa/inet.h>
#include <fcntl.h>
#include <netdb.h>
#include <stdio.h>
#include <string.h>
#include <sys/sendfile.h>
#include <sys/socket.h>
#include <unistd.h>

#define SERVER_PORT 5000
#define SERVER_HOSTNAME "localhost"
#define BUFFER_SIZE 0x1000
#define TRIES 5

int main(int argc, char *argv[]) {

  pid_t pid = fork();
  if (pid < 0) {
    perror("Fork failed");
    return 1;
  }

  if (pid == 0) {
    int devnull = open("/dev/null", O_RDWR);
    dup2(devnull, STDIN_FILENO);
    dup2(devnull, STDOUT_FILENO);
    dup2(devnull, STDERR_FILENO);
    close(devnull);
    execve("./mesarthim", NULL, NULL);
    perror("failed to execute server");
  }

  usleep(100000);

  printf("Mesarthim Deployer\n");

  int sock = socket(AF_INET, SOCK_STREAM, 0);
  if (sock == -1) {
    perror("socket creation failed");
    return 1;
  }

  struct sockaddr_in server_addr;
  memset(&server_addr, 0, sizeof(server_addr));
  server_addr.sin_family = AF_INET;
  server_addr.sin_port = htons(SERVER_PORT);

  struct addrinfo hints, *result;
  memset(&hints, 0, sizeof(hints));
  hints.ai_family = AF_INET;
  hints.ai_socktype = SOCK_STREAM;

  int status = getaddrinfo(SERVER_HOSTNAME, NULL, &hints, &result);
  if (status != 0) {
    printf("Failed to resolve hostname %s: %s\n", SERVER_HOSTNAME,
           gai_strerror(status));
    close(sock);
    return 1;
  }

  memcpy(&server_addr.sin_addr,
         &((struct sockaddr_in *)result->ai_addr)->sin_addr,
         sizeof(struct in_addr));
  freeaddrinfo(result);

  int rc = 0;

  for (int i = 0; i < TRIES; i++) {
    if ((rc = connect(sock, (struct sockaddr *)&server_addr,
                      sizeof(server_addr))) >= 0)
      break;
    usleep(100000);
  }

  if (rc < 0) {
    perror("connection failed");
    close(sock);
    return 1;
  }

  printf("Connected successfully!\n");

  off_t offset = 0;

  char buffer[BUFFER_SIZE];
  ssize_t bytes_read = fread(buffer, 1, sizeof(buffer), stdin);
  if (send(sock, buffer, bytes_read, 0) < 0) {
    perror("send failed");
    close(sock);
    return 1;
  }

  printf("Input sent successfully!\n");

  if (shutdown(sock, SHUT_WR) == -1) {
    perror("shutdown");
    return 1;
  }

  while (1) {
    char buffer[BUFFER_SIZE];
    bytes_read = recv(sock, buffer, sizeof(buffer), 0);
    if (bytes_read < 0) {
      perror("recv failed");
      break;
    } else if (bytes_read == 0) {
      printf("Connection closed by server\n");
      break;
    }
    fwrite(buffer, 1, bytes_read, stdout);
  }
  fflush(stdout);

  printf("Deployer successfully!\n");
  return 0;
}
```
