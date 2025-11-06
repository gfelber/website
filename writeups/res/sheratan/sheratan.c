```c
#include "sheratan.h"

bool sheratan_status_init = false;
struct sheratan_cmd *top_cmd = NULL;
struct sheratan_status status = {};

static long sheratan_ioctl(struct file *filp, unsigned int cmd,
                           unsigned long arg) {
  struct sheratan_param param;
  struct sheratan_cmd *current_cmd, **next_cmd;

  if (copy_from_user(&param, (void __user *)arg, sizeof(param)))
    return -EFAULT;

  switch (cmd) {
  case SHERATAN_IOCTL_PUSH:
    pr_info("sheratan: push\n");
    if (param.cmd < PING || param.cmd > BOOST_DISABLE)
      return -EINVAL;
    current_cmd = kzalloc(sizeof(struct sheratan_cmd), GFP_KERNEL_ACCOUNT);
    if (!current_cmd)
      return -ENOMEM;
    current_cmd->cmd = param.cmd;
    init_completion(&current_cmd->done);
    for (next_cmd = &top_cmd; *next_cmd; next_cmd = &(*next_cmd)->next)
      ;
    *next_cmd = current_cmd;
    wait_for_completion(&current_cmd->done);
    return 0;

  case SHERATAN_IOCTL_POP:
    pr_info("sheratan: pop\n");
    if (top_cmd == NULL)
      return -EINVAL;
    if (copy_to_user((void __user *)arg, &top_cmd->cmd, sizeof(top_cmd->cmd)))
      return -EFAULT;
    filp->private_data = top_cmd;
    top_cmd = top_cmd->next;
    return 0;

  case SHERATAN_IOCTL_DONE:
    pr_info("sheratan: done\n");
    if (!filp->private_data)
      return -EINVAL;
    current_cmd = filp->private_data;
    complete(&current_cmd->done);
    kfree(current_cmd);
    filp->private_data = NULL;
    return 0;

  case SHERATAN_IOCTL_SET_STATUS:
    pr_info("sheratan: set status\n");
    if (copy_from_user(&status, param.status, sizeof(status)))
      return -EFAULT;
    sheratan_status_init = true;
    return 0;

  case SHERATAN_IOCTL_GET_STATUS:
    pr_info("sheratan: get status\n");
    if (!sheratan_status_init)
      return -EINVAL;
    if (copy_to_user(param.status, &status, sizeof(status)))
      return -EFAULT;
    return 0;
  }

  return -EINVAL;
}

static int sheratan_open(struct inode *inode, struct file *file) {
  pr_info("sheratan: open\n");
  file->private_data = NULL;
  return 0;
}

static const struct proc_ops sheratan_fops = {
    .proc_ioctl = sheratan_ioctl,
    .proc_open = sheratan_open,
};

static int sheratan_init(void) {
  pr_info("sheratan: init\n");
  proc_create("sheratan", 0666, NULL, &sheratan_fops);
  return 0;
}

static void sheratan_exit(void) {
  pr_info("sheratan: exit\n");
  remove_proc_entry("sheratan", NULL);
}

module_init(sheratan_init);
module_exit(sheratan_exit);

MODULE_AUTHOR("\x06\xfe\x1b\xe2");
MODULE_DESCRIPTION("sheratan");
MODULE_LICENSE("GPL");
```
