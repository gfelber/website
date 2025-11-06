```c
#include <arpa/inet.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <unistd.h>

#include "pb/mesarthim.pb-c.h"

#define forever for (;;)
#define SERVER_PORT 5000
#define SERVER_IP "192.168.111.1"
#define FRAME_HEADER 5
#define PROC_NAME "/proc/sheratan"

enum sheratan_cmds {
  PING = 1,
  REBOOT = 2,
  BOOST_ENABLE = 3,
  BOOST_DISABLE = 4
};

struct sheratan_telemetry {
  float latitude;
  float longitude;
  int altitude;
  float velocity;
  float inclination;
  float period;
};

struct sheratan_status {
  char name[0x10];
  int time;
  int uptime;
  int signal;
  int latency;
  bool boost;
  struct sheratan_telemetry telemetry;
};

struct sheratan_param {
  union {
    struct sheratan_status *status;
    enum sheratan_cmds cmd;
  };
};

#define SHERATAN_IOCTL_PUSH _IOW('A', 0, struct sheratan_param *)
#define SHERATAN_IOCTL_POP _IOW('A', 1, struct sheratan_param *)
#define SHERATAN_IOCTL_DONE _IOW('A', 2, struct sheratan_param *)
#define SHERATAN_IOCTL_SET_STATUS _IOW('A', 3, struct sheratan_param *)
#define SHERATAN_IOCTL_GET_STATUS _IOW('A', 4, struct sheratan_param *)

int shfd;

int init_sheratan() {
  int fd = open(PROC_NAME, O_RDWR);
  if (fd < 0) {
    perror("Failed to open sheratan device");
    return -1;
  }
  return fd;
}

enum sheratan_cmds sheratan_pop(int sheratan_fd) {
  struct sheratan_param param;
  int ret = ioctl(sheratan_fd, SHERATAN_IOCTL_POP, &param);
  if (ret < 0)
    return -1;
  return param.cmd;
}

void sheratan_done(int sheratan_fd) {
  struct sheratan_param param;
  if (ioctl(sheratan_fd, SHERATAN_IOCTL_DONE, &param) < 0) {
    perror("Failed to execute sheratan_done ioctl");
  }
}

void sheratan_set_status(int sheratan_fd, struct sheratan_status *status) {
  struct sheratan_param param = {.status = status};
  if (ioctl(sheratan_fd, SHERATAN_IOCTL_SET_STATUS, &param) < 0) {
    perror("Failed to set sheratan status");
  }
}

int recv_size(int fd, void *buf, size_t size, int flags) {
  size_t total_received = 0;
  while (total_received < size) {
    ssize_t bytes_received =
        recv(fd, (char *)buf + total_received, size - total_received, flags);
    if (bytes_received <= 0) {
      return bytes_received;
    }
    total_received += bytes_received;
  }
  return total_received;
}

Main__Frame *create_command_frame(Main__Commands cmd) {
  // Create frame
  Main__Frame *frame = calloc(1, sizeof(Main__Frame));
  main__frame__init(frame);
  frame->cmd = cmd;
  frame->status = NULL; // Client doesn't send status

  // Calculate size
  int32_t size;
  while (frame->size != (size = main__frame__get_packed_size(frame)))
    frame->size = size;

  return frame;
}

void free_frame(Main__Frame *frame) {
  if (!frame)
    return;

  if (frame->status) {
    if (frame->status->name) {
      free(frame->status->name);
    }
    if (frame->status->telemetry) {
      free(frame->status->telemetry);
    }
    free(frame->status);
  }

  free(frame);
}

Main__Frame *get_response(int sock) {
  Main__FrameHeader *header;
  unsigned char *buffer = malloc(FRAME_HEADER);
  size_t bytes_received = 0;
  for (size_t i = 0; i < FRAME_HEADER; i++) {
    bytes_received += recv(sock, buffer + i, 1, 0);
    if (bytes_received != (i + 1)) {
      perror("recv failed");
      printf("Client disconnected\n");
      break;
    }

    header = main__frame__header__unpack(NULL, bytes_received, buffer);
    if (header)
      break;
  }

  if (!header) {
    printf("Failed to unpack FrameHeader\n");
    free(buffer);
    close(sock);
    return NULL;
  }

  buffer = realloc(buffer, header->size);
  if (!buffer) {
    printf("Failed to reallocate buffer\n");
    free(buffer);
    main__frame__header__free_unpacked(header, NULL);
    close(sock);
    return NULL;
  }

  int recv_result = recv_size(sock, buffer + bytes_received,
                              header->size - bytes_received, 0);
  if (recv_result <= 0) {
    perror("recv_size failed");
    free(buffer);
    main__frame__header__free_unpacked(header, NULL);
    close(sock);
    return NULL;
  }
  main__frame__header__free_unpacked(header, NULL);

  bytes_received += recv_result;

  if (bytes_received != header->size) {
    printf("Incomplete data received: expected %u, got %zu\n", header->size,
           bytes_received);
    free(buffer);
    main__frame__header__free_unpacked(header, NULL);
    close(sock);
    return NULL;
  }
  Main__Frame *result = main__frame__unpack(NULL, bytes_received, buffer);
  return result;
}

void update_status(int sock) {
  struct sheratan_status status;
  Main__Frame *frame = get_response(sock);
  strcpy(status.name, frame->status->name);
  status.time = frame->status->time;
  status.uptime = frame->status->uptime;
  status.signal = frame->status->signal;
  status.latency = frame->status->latency;
  status.boost = frame->status->boost;
  status.telemetry.latitude = frame->status->telemetry->latitude;
  status.telemetry.longitude = frame->status->telemetry->longitude;
  status.telemetry.altitude = frame->status->telemetry->altitude;
  status.telemetry.velocity = frame->status->telemetry->velocity;
  status.telemetry.inclination = frame->status->telemetry->inclination;
  status.telemetry.period = frame->status->telemetry->period;
  sheratan_set_status(shfd, &status);

  main__frame__free_unpacked(frame, NULL);
}

int send_cmd(int sock, Main__Commands cmd) {
  // Create and pack command frame
  Main__Frame *frame = create_command_frame(cmd);
  size_t frame_size = main__frame__get_packed_size(frame);
  uint8_t *buffer = malloc(frame_size);

  if (!buffer) {
    printf("Failed to allocate buffer\n");
    free_frame(frame);
    close(sock);
    return -1;
  }

  size_t packed_len = main__frame__pack(frame, buffer);

  if (send(sock, buffer, packed_len, 0) < 0) {
    perror("send failed");
    free(buffer);
    free_frame(frame);
    close(sock);
    return -1;
  }
  free(buffer);
  free_frame(frame);

  return 0;
}

void run_cmd(Main__Commands cmd) {
  // Create socket
  int sock = socket(AF_INET, SOCK_STREAM, 0);
  if (sock == -1) {
    perror("socket creation failed");
    return;
  }

  // Configure server address
  struct sockaddr_in server_addr;
  memset(&server_addr, 0, sizeof(server_addr));
  server_addr.sin_family = AF_INET;
  server_addr.sin_port = htons(SERVER_PORT);

  if (inet_pton(AF_INET, SERVER_IP, &server_addr.sin_addr) <= 0) {
    printf("Invalid address/ Address not supported\n");
    close(sock);
    return;
  }

  // Connect to server
  if (connect(sock, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
    perror("connection failed");
    close(sock);
    return;
  }

  if (send_cmd(sock, cmd) < 0) {
    close(sock);
    return;
  }

  // update status on every command
  if (send_cmd(sock, MAIN__COMMANDS__STATUS) < 0) {
    close(sock);
    return;
  }

  // done sending
  shutdown(sock, SHUT_WR);

  Main__Frame *frame = get_response(sock);
  if (frame)
    main__frame__free_unpacked(frame, NULL);
  sheratan_done(shfd);
  update_status(sock);

  close(sock);
}

int main() {
  shfd = init_sheratan();

  forever {
    Main__Commands cmd = -1;
    while (cmd == -1) {
      usleep(100000);
      cmd = sheratan_pop(shfd);
    }
    run_cmd(cmd);
  }

  return 0;
}
```
