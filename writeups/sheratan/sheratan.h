```c
#ifndef _SHERATAN_H
#define _SHERATAN_H

#include <linux/completion.h>
#include <linux/init.h>
#include <linux/module.h>
#include <linux/proc_fs.h>

#define DEVICE_NAME "sheratan"
#define CLASS_NAME DEVICE_NAME

enum sheratan_cmds {
  PING = 1,
  REBOOT = 2,
  BOOST_ENABLE = 3,
  BOOST_DISABLE = 4
};

struct sheratan_cmd {
  struct sheratan_cmd *next;
  enum sheratan_cmds cmd;
  struct completion done;
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
    struct sheratan_status __user *status;
    enum sheratan_cmds cmd;
  };
};

#define SHERATAN_IOCTL_PUSH _IOW('A', 0, struct sheratan_param *)
#define SHERATAN_IOCTL_POP _IOW('A', 1, struct sheratan_param *)
#define SHERATAN_IOCTL_DONE _IOW('A', 2, struct sheratan_param *)
#define SHERATAN_IOCTL_SET_STATUS _IOW('A', 3, struct sheratan_param *)
#define SHERATAN_IOCTL_GET_STATUS _IOW('A', 4, struct sheratan_param *)

#endif
```
