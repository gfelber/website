```c
#include "pb/mesarthim.pb-c.h"
#include <arpa/inet.h>

#include <math.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#include "pb/mesarthim.pb-c.h"

#define PORT 5000
#define BACKLOG 10
#define FRAME_HEADER 5
static const time_t EPOCH_START_TIME = 0;

#define BASE_ALTITUDE 550
#define GM 398600.4418
#define EARTH_RADIUS 6371.0

int start;

char g_name[0x10] = "mesarthim";

Main__Telemetry g_telemetry = {
    .base = PROTOBUF_C_MESSAGE_INIT(&main__telemetry__descriptor),
    .latitude = 52.21817869061595,
    .longitude = 4.420064402798399,
    .altitude = 550000,
    .velocity = 7.8,
    .inclination = 53.0,
    .period = 96.3};

Main__Status g_status = {
    .base = PROTOBUF_C_MESSAGE_INIT(&main__status__descriptor),
    .name = g_name,
    .time = 0,
    .uptime = 0,
    .signal = 0,
    .latency = 0,
    .telemetry = &g_telemetry,
};

void update_status() {
  struct timeval tv;
  gettimeofday(&tv, NULL);

  time_t now_sec = tv.tv_sec;
  double now_usec = (double)tv.tv_sec + (double)tv.tv_usec / 1000000.0;

  g_status.time = (int32_t)now_sec;

  // Calculate uptime from fixed epoch
  g_status.uptime = (int32_t)(now_sec - start);

  // Calculate signal with boost effects and variation (using microsecond
  // precision)
  int base_signal = g_status.boost ? -70 : -75;
  g_status.signal = base_signal + ((long)(now_usec * 1000000) % 20) - 10;

  // Calculate latency with variation (using microsecond precision)
  g_status.latency = 24 + ((long)(now_usec * 1000000) % 16) - 8;

  // Calculate realistic orbital parameters (using microsecond precision)
  g_status.telemetry->altitude =
      (BASE_ALTITUDE + ((long)(now_usec * 1000000) % 6) - 3) * 1000;

  // Calculate enhanced orbital velocity
  double orbital_radius =
      (g_status.telemetry->altitude / 1000.0) + EARTH_RADIUS; // km
  double realistic_velocity = sqrt(GM / orbital_radius);      // km/s
  double multiplier = 1000.0;
  g_status.telemetry->velocity =
      realistic_velocity * multiplier; // Enhanced speed for visibility

  // Calculate orbital period: T = 2π * sqrt(r³/GM)
  double period_seconds = 2 * M_PI * sqrt(pow(orbital_radius, 3) / GM);
  g_status.telemetry->period = period_seconds / 60.0; // Convert to minutes

  // Calculate satellite position (great circle orbit through Netherlands)
  // Angular velocity calculation with enhanced speed based on elapsed time
  // (microsecond precision)
  double elapsed_time = now_usec - (double)EPOCH_START_TIME;
  double orbit_angle = elapsed_time * 0.000549 * multiplier;

  // Origin coordinates (Netherlands) - great circle passes through here
  double origin_lat = 52.21817869061595 * M_PI / 180.0; // Convert to radians
  double origin_lng = 4.420064402798399 * M_PI / 180.0;

  // Great circle parameters
  double orbit_position = orbit_angle * 0.01; // Position along the great circle
  double bearing = 120.0 * M_PI / 180.0;

  // Great circle calculation using spherical trigonometry
  double distance = orbit_position; // Angular distance

  // Calculate position along great circle using spherical trigonometry
  double sat_lat_rad = asin(sin(origin_lat) * cos(distance) +
                            cos(origin_lat) * sin(distance) * cos(bearing));

  double dlon = atan2(sin(bearing) * sin(distance) * cos(origin_lat),
                      cos(distance) - sin(origin_lat) * sin(sat_lat_rad));
  double sat_lng_rad = origin_lng + dlon;

  // Convert back to degrees
  g_status.telemetry->latitude = sat_lat_rad * 180.0 / M_PI;
  g_status.telemetry->longitude = sat_lng_rad * 180.0 / M_PI;

  // Normalize longitude
  while (g_status.telemetry->longitude > 180.0)
    g_status.telemetry->longitude -= 360.0;
  while (g_status.telemetry->longitude < -180.0)
    g_status.telemetry->longitude += 360.0;

  // Inclination with small realistic variation (using microsecond precision)
  g_status.telemetry->inclination =
      53.0 + (sin(now_usec * 0.001) * 0.1); // ±0.1° variation
}

Main__Frame *make_response_frame(Main__Frame *response, Main__Commands cmd) {

  response->cmd = cmd;

  if (cmd == MAIN__COMMANDS__STATUS) {
    update_status();
    response->status = &g_status;
  }

  // Calculate size
  response->size = main__frame__get_packed_size(response);

  return response;
}

Main__Commands handle_command(Main__Frame *frame) {
  switch (frame->cmd) {
  case MAIN__COMMANDS__PING:
    printf("PING: Satellite is alive\n");
    return MAIN__COMMANDS__PONG;
  case MAIN__COMMANDS__PONG:
    printf("PING: Satellite is alive\n");
    return MAIN__COMMANDS__PING;
  case MAIN__COMMANDS__REBOOT:
    printf("REBOOT: System reboot initiated      - restarting\n");
    return MAIN__COMMANDS__REBOOT;
  case MAIN__COMMANDS__BOOST_ENABLE:
    printf("BOOST_ENABLE: Signal boost ENABLED   - Power increased by 5dBm\n");
    g_status.boost = 1;
    return MAIN__COMMANDS__BOOST_ENABLE;
  case MAIN__COMMANDS__BOOST_DISABLE:
    printf("BOOST_DISABLE: Signal boost DISABLED - Power decreased by 5dBm\n");
    g_status.boost = 0;
    return MAIN__COMMANDS__BOOST_DISABLE;
  case MAIN__COMMANDS__STATUS:
    printf("STATUS: Returning current satellite status\n");
    return MAIN__COMMANDS__STATUS;
  case MAIN__COMMANDS__SET_NAME:
    if (frame->status && frame->status->name) {
      strncpy(g_name, frame->status->name, sizeof(g_name) - 1);
      printf("SET_NAME: Changing satellite name to '%s'\n", g_name);
      return MAIN__COMMANDS__SET_NAME;
    }
    printf("SET_NAME: No name provided, ignoring\n");
    return MAIN__COMMANDS__ERROR;

  default:
    printf("Unknown command: %d\n", frame->cmd);
    return MAIN__COMMANDS__ERROR;
  }
}

void free_frame(Main__Frame *frame) {
  if (!frame)
    return;

  if (frame->status == &g_status)
    frame->status = NULL;

  main__frame__free_unpacked(frame, NULL);
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

int handle_client(int client_fd) {
  void *buffer = malloc(FRAME_HEADER);
  ssize_t bytes_received;
  int rc = 0;

  printf("New client connected (fd: %d)\n", client_fd);

  while (1) {
    Main__FrameHeader *header = NULL;
    bytes_received = 0;

    for (size_t i = 0; i < FRAME_HEADER; i++) {
      bytes_received += recv(client_fd, buffer + i, 1, 0);
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
      break;
    }

    printf("Expected frame size: %u bytes\n", header->size);
    buffer = realloc(buffer, header->size);

    bytes_received += recv_size(client_fd, buffer + bytes_received,
                                header->size - bytes_received, 0);

    if (bytes_received != header->size) {
      perror("recv failed");
      main__frame__header__free_unpacked(header, NULL);
      printf("Client disconnected\n");
      break;
    }
    main__frame__header__free_unpacked(header, NULL);

    // Unpack the Frame message
    Main__Frame *frame = main__frame__unpack(NULL, bytes_received, buffer);
    if (!frame) {
      printf("Failed to unpack Frame message\n");
      continue;
    }

    printf("Frame size: %u\n", frame->size);

    if (frame->cmd) {
      // Create response
      frame = make_response_frame(frame, handle_command(frame));

      // Pack and send response
      uint8_t *frame_buffer = malloc(frame->size);

      if (frame_buffer) {
        size_t packed_len = main__frame__pack(frame, frame_buffer);

        if (send(client_fd, frame_buffer, packed_len, 0) < 0) {
          perror("send failed");
        }

        free(frame_buffer);
      }
    }

    Main__Commands cmd = frame->cmd;
    free_frame(frame);
    if (cmd == MAIN__COMMANDS__REBOOT) {
      rc = 1;
      break;
    }
  }

  free(buffer);
  return rc;
}

int main() {
  int server_fd, client_fd;
  struct sockaddr_in server_addr, client_addr;
  socklen_t client_addr_len = sizeof(client_addr);
  start = (int)time(NULL);
  setbuf(stdin, NULL);
  setbuf(stdout, NULL);
  setbuf(stderr, NULL);

  printf("Mesarthim Satellite TCP Server\n");

  srand(time(NULL));

  server_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (server_fd == -1) {
    perror("socket creation failed");
    return 1;
  }

  int opt = 1;
  if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
    perror("setsockopt failed");
    close(server_fd);
    return 1;
  }

  memset(&server_addr, 0, sizeof(server_addr));
  server_addr.sin_family = AF_INET;
  server_addr.sin_addr.s_addr = INADDR_ANY;
  server_addr.sin_port = htons(PORT);

  if (bind(server_fd, (struct sockaddr *)&server_addr, sizeof(server_addr)) <
      0) {
    perror("bind failed");
    close(server_fd);
    return 1;
  }

  if (listen(server_fd, BACKLOG) < 0) {
    perror("listen failed");
    close(server_fd);
    return 1;
  }

  printf("Server listening on port %d\n", PORT);
  printf("Initial status: %s, Signal: %d%%, Latency: %dms\n", g_status.name,
         g_status.signal, g_status.latency);

  while (1) {
    client_fd =
        accept(server_fd, (struct sockaddr *)&client_addr, &client_addr_len);
    if (client_fd < 0) {
      perror("accept failed");
      continue;
    }

    if (handle_client(client_fd)) {
      close(client_fd);
      system("reboot-aries");
      break;
    }
    close(client_fd);
  }

  close(server_fd);
  return 0;
}
```
