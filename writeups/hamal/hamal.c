```c
#include <arpa/inet.h>
#include <fcgi_stdio.h>
#include <fcntl.h>
#include <malloc.h>
#include <math.h>
#include <netinet/in.h>
#include <openssl/bio.h>
#include <openssl/buffer.h>
#include <openssl/evp.h>
#include <openssl/sha.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <sys/ioctl.h>
#include <sys/time.h>
#include <unistd.h>

#define PROC_NAME "/proc/sheratan"

#define GM 398600.4418
#define EARTH_RADIUS 6371.0

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

struct ws_frame {
  unsigned char fin;
  unsigned char opcode;
  size_t length;
  char *data;
};

struct sheratan_status g_status = {.name = "mesarthim",
                                   .time = 0,
                                   .uptime = 0,
                                   .signal = 0,
                                   .latency = 0,
                                   .boost = 0,
                                   .telemetry = {.latitude = 52.21817869061595,
                                                 .longitude = 4.420064402798399,
                                                 .altitude = 550000,
                                                 .velocity = 7.8,
                                                 .inclination = 53.0,
                                                 .period = 96.3}};

enum OPCODES {
  CONTINUATION = 0x0,
  TEXT = 0x1,
  BINARY = 0x2,
  CLOSE = 0x8,
  WS_PING = 0x9,
  WS_PONG = 0xA
};

#define SHERATAN_IOCTL_PUSH _IOW('A', 0, struct sheratan_param *)
#define SHERATAN_IOCTL_POP _IOW('A', 1, struct sheratan_param *)
#define SHERATAN_IOCTL_DONE _IOW('A', 2, struct sheratan_param *)
#define SHERATAN_IOCTL_SET_STATUS _IOW('A', 3, struct sheratan_param *)
#define SHERATAN_IOCTL_GET_STATUS _IOW('A', 4, struct sheratan_param *)

int shfd;
int init_sheratan() { return open(PROC_NAME, O_RDWR); }

int sheratan_push(int sheratan_fd, enum sheratan_cmds cmd) {
  struct sheratan_param param = {.cmd = cmd};
  return ioctl(sheratan_fd, SHERATAN_IOCTL_PUSH, &param);
}

int sheratan_get_status(int sheratan_fd, struct sheratan_status *status) {
  struct sheratan_param param = {.status = status};
  return ioctl(sheratan_fd, SHERATAN_IOCTL_GET_STATUS, &param);
}

const struct ws_frame close_frame = {1, CLOSE, 0, NULL};
#define CLOSE_FRAME ((struct ws_frame *)&close_frame)

static inline char *get_env(const char *name) {
  char *value = getenv(name);
  return value ? value : "";
}

const char *status[] = {[200] = "OK",
                        [400] = "Bad Request",
                        [404] = "Not Found",
                        [500] = "Internal Server Error"};

static void send_response(const char *content_type, int status_code,
                          const char *body, ...) {
  va_list args;
  va_start(args, body);
  if (status_code != 200) {
    fprintf(stdout, "Status: %d %s\r\n", status_code, status[status_code]);
  }
  fprintf(stdout, "Content-Type: %s\r\n\r\n", content_type);
  if (body)
    vfprintf(stdout, body, args);
  va_end(args);
}

static void send_text(const char *text) {
  send_response("text/plain", 200, "%s\n", text);
}

static char *base64_encode(const unsigned char *input, int length) {
  BIO *bmem, *b64;
  BUF_MEM *bptr;

  b64 = BIO_new(BIO_f_base64());
  bmem = BIO_new(BIO_s_mem());
  b64 = BIO_push(b64, bmem);
  BIO_set_flags(b64, BIO_FLAGS_BASE64_NO_NL);
  BIO_write(b64, input, length);
  BIO_flush(b64);
  BIO_get_mem_ptr(b64, &bptr);

  char *buff = malloc(bptr->length + 1);
  memcpy(buff, bptr->data, bptr->length);
  buff[bptr->length] = 0;

  BIO_free_all(b64);
  return buff;
}

static int ws_upgrade(void) {
  char *key = get_env("HTTP_SEC_WEBSOCKET_KEY");
  if (!key || strlen(key) == 0) {
    send_response("text/plain", 400, "Bad Request");
    return 0;
  }

  char guid[] = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
  char concat[256];
  snprintf(concat, sizeof(concat), "%s%s", key, guid);

  unsigned char hash[SHA_DIGEST_LENGTH];
  SHA1((unsigned char *)concat, strlen(concat), hash);

  char *accept_b64 = base64_encode(hash, SHA_DIGEST_LENGTH);

  char *scheme =
      (strcmp(get_env("REQUEST_SCHEME"), "https") == 0) ? "wss" : "ws";
  char *host = get_env("HTTP_HOST");
  char *urlpath = get_env("SCRIPT_NAME");

  fprintf(stdout, "Status: 101\r\n");
  fprintf(stdout, "Upgrade: WebSocket\r\n");
  fprintf(stdout, "Connection: Upgrade\r\n");
  fprintf(stdout, "Sec-WebSocket-Location: %s://%s%s\r\n", scheme, host,
          urlpath);
  fprintf(stdout, "Sec-WebSocket-Accept: %s\r\n\r\n", accept_b64);
  fflush(stdout);

  free(accept_b64);
  return 1;
}

static inline void free_frame(struct ws_frame *frame) {
  if (frame->data)
    free(frame->data);
  free(frame);
}

static void ws_send(struct ws_frame *frame) {
  unsigned char header[10];
  int header_len = 0;

  header[0] = (frame->fin ? 0x80 : 0) | (frame->opcode & 0x0F);
  header_len++;

  if (frame->length < 126) {
    header[1] = frame->length;
    header_len++;
  } else if (frame->length < 65536) {
    header[1] = 126;
    *(short *)&header[2] = htons(frame->length);
    header_len += 3;
  } else {
    header[1] = 127;
    *(long *)&header[2] = htonl(frame->length);
    header_len += 9;
  }

  fwrite(header, 1, header_len, stdout);
  if (frame->length > 0)
    fwrite(frame->data, 1, frame->length, stdout);
  fflush(stdout);
}

static struct ws_frame *ws_recv(void) {
  unsigned char header[2];
  struct ws_frame *frame;

  if (fread(header, 1, 2, stdin) != 2) {
    ws_send(CLOSE_FRAME);
    return NULL;
  }

  frame = calloc(1, sizeof(struct ws_frame));

  frame->fin = header[0] & 0x80;
  frame->opcode = header[0] & 0x0F;

  if (frame->opcode == CLOSE) {
    ws_send(CLOSE_FRAME);
    free(frame);
    return NULL;
  }

  frame->length = header[1] & 0x7F;
  int mask_bit = header[1] & 0x80;

  if (frame->length == 126) {
    fread(&frame->length, 1, 2, stdin);
    frame->length = ntohs(frame->length);
  } else if (frame->length == 127) {
    fread((char *)&frame->length, 1, 8, stdin);
    frame->length = ntohl(frame->length);
  }

  unsigned char masking_key[4];
  if (mask_bit)
    fread(masking_key, 1, 4, stdin);

  frame->data = malloc(frame->length);
  if (frame->length > 0) {
    fread(frame->data, 1, frame->length, stdin);

    if (mask_bit)
      for (size_t i = 0; i < frame->length; i++)
        frame->data[i] ^= masking_key[i % 4];
  }

  return frame;
}

void update_status() {
  sheratan_get_status(shfd, &g_status);

  // simulate gap
  struct timeval tv;
  gettimeofday(&tv, NULL);

  time_t now_sec = tv.tv_sec;
  double now_usec = (double)tv.tv_sec + (double)tv.tv_usec / 1000000.0;

  // Calculate uptime from fixed epoch
  g_status.uptime += (int32_t)(now_sec - g_status.time);

  // Calculate signal with boost effects and variation (using microsecond
  // precision)
  int base_signal = g_status.boost ? -70 : -75;
  g_status.signal = base_signal + ((long)(now_usec * 1000000) % 20) - 10;

  // Calculate latency with variation (using microsecond precision)
  g_status.latency = 24 + ((long)(now_usec * 1000000) % 16) - 8;

  // Calculate realistic orbital parameters (using microsecond precision)
  g_status.telemetry.altitude += (((long)(now_usec * 1000000) % 6) - 3) * 1000;

  // Calculate enhanced orbital velocity
  double orbital_radius =
      (g_status.telemetry.altitude / 1000.0) + EARTH_RADIUS; // km
  double realistic_velocity = sqrt(GM / orbital_radius);     // km/s
  double multiplier = 1000.0;
  g_status.telemetry.velocity =
      realistic_velocity * multiplier; // Enhanced speed for visibility

  // Calculate orbital period: T = 2π * sqrt(r³/GM)
  double period_seconds = 2 * M_PI * sqrt(pow(orbital_radius, 3) / GM);
  g_status.telemetry.period = period_seconds / 60.0; // Convert to minutes

  // Calculate satellite position (great circle orbit through Netherlands)
  // Angular velocity calculation with enhanced speed based on elapsed time
  // (microsecond precision)
  double elapsed_time = now_usec - (double)g_status.time;
  double orbit_angle = elapsed_time * 0.000549 * multiplier;

  // Treat this as an angular distance in radians (your previous
  // "orbit_position")
  double distance = orbit_angle * 0.01;

  // Constant bearing (in radians)
  double bearing = 120.0 * M_PI / 180.0;

  // Current position in radians
  double lat1 = g_status.telemetry.latitude * M_PI / 180.0;
  double lon1 = g_status.telemetry.longitude * M_PI / 180.0;

  // Rhumb-line forward calculation
  double dphi = distance * cos(bearing);
  double phi2 = lat1 + dphi;

  // Handle pole crossing (optional guard)
  if (phi2 > M_PI / 2)
    phi2 = M_PI - phi2;
  if (phi2 < -M_PI / 2)
    phi2 = -M_PI - phi2;

  // Mercator latitude difference
  double dpsi =
      log(tan(M_PI / 4.0 + phi2 / 2.0) / tan(M_PI / 4.0 + lat1 / 2.0));
  double q = (fabs(dpsi) > 1e-12) ? (dphi / dpsi) : cos(lat1);

  double dlon = distance * sin(bearing) / q;
  double lon2 = lon1 + dlon;

  // Normalize longitude to [-pi, pi)
  while (lon2 >= M_PI)
    lon2 -= 2.0 * M_PI;
  while (lon2 < -M_PI)
    lon2 += 2.0 * M_PI;

  // Convert back to degrees
  g_status.telemetry.latitude = phi2 * 180.0 / M_PI;
  g_status.telemetry.longitude = lon2 * 180.0 / M_PI;

  // Normalize longitude
  while (g_status.telemetry.longitude > 180.0)
    g_status.telemetry.longitude -= 360.0;
  while (g_status.telemetry.longitude < -180.0)
    g_status.telemetry.longitude += 360.0;

  // Inclination with small realistic variation (using microsecond precision)
  g_status.telemetry.inclination += (sin(now_usec * 0.001) * 0.1);

  g_status.time = now_sec;
}

static void send_status(void) {

  char buf[512];
  update_status();
  char time_str[32];
  long time = g_status.time;

  strftime(time_str, sizeof(time_str), "%Y-%m-%d %H:%M:%S", localtime(&time));

  size_t length =
      snprintf(buf, sizeof(buf),
               "{"
               "\"name\":\"%s\","
               "\"time\":\"%s\","
               "\"uptime\":%d,"
               "\"signal\":%d,"
               "\"latency\":%d,"
               "\"boost\":%s,"
               "\"telemetry\":{"
               "\"latitude\":%.3f,"
               "\"longitude\":%.3f,"
               "\"altitude\":%d,"
               "\"velocity\":%.2f,"
               "\"inclination\":%.1f,"
               "\"period\":%.1f"
               "}"
               "}",
               g_status.name, time_str, g_status.uptime, g_status.signal,
               g_status.latency, g_status.boost ? "true" : "false",
               g_status.telemetry.latitude, g_status.telemetry.longitude,
               g_status.telemetry.altitude, g_status.telemetry.velocity,
               g_status.telemetry.inclination, g_status.telemetry.period);

  struct ws_frame status_frame = {0x80, TEXT, length, buf};
  ws_send(&status_frame);
}

static void handle_status(void) {
  while (1) {
    struct ws_frame *frame = ws_recv();
    if (!frame)
      break;

    switch (frame->opcode) {
    case WS_PING:
    case WS_PONG:
      frame->opcode ^= WS_PING ^ WS_PONG;
      ws_send(frame);
      free_frame(frame);
      continue;
    case TEXT:
      if (strncmp(frame->data, "status", 6) == 0) {
        send_status();
        free_frame(frame);
        break;
      }
    default:
      ws_send(CLOSE_FRAME);
      free_frame(frame);
      return;
    }
  }
}

static void handle_ping(void) {
  sheratan_push(shfd, PING);
  send_text("pong");
}

static void handle_reboot(void) {
  sheratan_push(shfd, REBOOT);
  send_text("System reboot initiated - ARIES services restarting");
}

static void handle_signal_boost(void) {
  int content_length = atoi(get_env("CONTENT_LENGTH"));
  char content[content_length];

  gets(content);

  if (strstr(content, "enable")) {
    sheratan_push(shfd, BOOST_ENABLE);
    send_text("Signal boost ENABLED - Power increased by +5dBm");
  } else if (strstr(content, "disable")) {
    sheratan_push(shfd, BOOST_DISABLE);
    send_text("Signal boost DISABLED - Power restored to normal");
  } else {
    send_response("text/plain", 400, "Invalid signal boost command\n");
  }
}

int main(void) {
  shfd = init_sheratan();

  g_status.time = time(NULL);

  while (FCGI_Accept() == 0) {

    char *path_info = get_env("PATH_INFO");
    char *upgrade = get_env("HTTP_UPGRADE");

    if (strcmp(upgrade, "websocket") == 0) {
      if (!ws_upgrade()) {
        FCGI_Finish();
        continue;
      }
    }

    if (strcmp(path_info, "/status") == 0) {
      handle_status();
    } else if (strcmp(path_info, "/ping") == 0) {
      handle_ping();
    } else if (strcmp(path_info, "/reboot") == 0) {
      handle_reboot();
    } else if (strcmp(path_info, "/boost") == 0) {
      handle_signal_boost();
    } else {
      send_response("text/plain", 404, "Path requested: %s", path_info);
    }

    FCGI_Finish();
  }

  return 0;
}
```
