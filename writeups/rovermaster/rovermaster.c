```c
// gcc main.c --no-pie -static -o main
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <unistd.h>

void cmd_get_planet(void);
void cmd_set_planet(void);
void cmd_get_name(void);
void cmd_set_name(void);
void cmd_move_rover(void);
void cmd_full_info(void);

struct rover {
    char weight;
    char x;
    char y;
    char z;
    char battery;
    char temperature;
    char planet[256];
    char name[256];
    void (*code)(void);
} typedef rover;

rover rovers[0xf] = {
    {0x96,0x00,0x00,0x00,0x19,0x64,"Mars","Curiosity 2.0", NULL},
    {0x78,0x0a,0x0f,0x01,0xf1,0x55,"Europa","Ice Explorer", NULL},
    {0xc8,0x14,0x05,0x00,0x82,0x4b,"Venus","Sulfur Trekker", NULL},
    {0x8c,0x05,0x14,0x00,0x0c,0x5a,"Titan","Methane Surfer", NULL},
    {0xa5,0x1e,0x0a,0x00,0x06,0x5f,"Pluto","Ice Miner", NULL},
    {0x82,0x00,0x19,0x00,0x2e,0x46,"Mercury","Solar Glider", NULL},
    {0xb4,0x0f,0x0f,0x00,0x78,0x50,"Neptune","Storm Navigator", NULL},
    {0x9b,0x19,0x00,0x00,0x14,0x41,"Moon","Lunar Walker", NULL},
    {0xbe,0x23,0x14,0x00,0x41,0x58,"Callisto","Ice Ranger", NULL},
    {0x6e,0x00,0x1e,0x00,0x3c,0x5c,"Venus","Cloud Dancer", NULL},
    {0xaf,0x28,0x05,0x00,0x01,0x4d,"Enceladus","Ice Fisher", NULL},
    {0xa0,0x0a,0x28,0x00,0x2d,0x53,"Mars","Dust Racer0", NULL},
    {0x91,0x14,0x19,0x00,0x51,0x4e,"Titan","Hydrocarbon Hunter", NULL},
    {0x82,0x2d,0x0a,0x00,0xf4,0x3c,"Io","Volcano Voyager", NULL},
    {0xaa,0x1e,0x23,0x00,0x5a,0x55,"Ganymede","Magnetic Mapper", NULL},
};
unsigned int current_rover = 0;

char *action_names[0x6] = {
    "Get Planet",
    "Set Planet",
    "Get Name",
    "Set Name",
    "Move rover",
    "Full info",
};
    
void (*actions[0x6])(void) = {
    cmd_get_planet,
	cmd_set_planet,
	cmd_get_name,
	cmd_set_name,
	cmd_move_rover,
	cmd_full_info
};

void die(char *msg)
{
  puts(msg);
  exit(1);
}

long long read_exactly(int fd, char *buf, size_t size)
{
  unsigned long long i = 0;
  size_t var;
  // BOF by 1
  do {
    if (size < i)
      return 0;
    var = read(fd,buf + i,1);
    if (var < 1)
      return -1;
    i = i + var;
  } while( true );
}

void cmd_get_planet(void)
{
  printf("Planet: %s\n",rovers[current_rover].planet);
}

void cmd_get_name(void)
{
  printf("Name: %s\n",rovers[current_rover].name);
}

void cmd_set_planet(void)
{
  int var;
  unsigned int size;
  printf("Send new planet size: ");
  var = scanf("%u",&size);

  if (var != 1) 
    die("err");

  if (0x100 < size) 
    die("Invalid planet len");

  printf("New planet: ");
  read_exactly(0,rovers[current_rover].name,size);
}

void cmd_set_name(void)
{
  int var;
  unsigned int size;
  printf("Send new name size: ");
  var = scanf("%u",&size);

  if (var != 1) 
    die("err");

  if (0x100 < size) 
    die("Invalid name len");

  printf("New name: ");
  read_exactly(0,rovers[current_rover].name,size);
}


void cmd_move_rover(void)
{
  int var;

  printf("Send coordinates (x y z): ");
  var = scanf("%hhu %hhu %hhu", &rovers[current_rover].x, &rovers[current_rover].y, &rovers[current_rover].z);
  if (var != 3)
    die("err");

  puts("Coordinates updated!");
}

void cmd_full_info(void)
{
  printf("Name: %s\n",rovers[current_rover].name);
  printf("Planet: %s\n",rovers[current_rover].planet);
  printf("Position (x, y, z): %hhu - %hhu - %hhu\n",
              rovers[current_rover].x,
              rovers[current_rover].y,
              rovers[current_rover].z);
  printf("Battery: %hhu%%\n",rovers[current_rover].battery);
  printf("Temperature: %hhu%%\n",rovers[current_rover].temperature);
  printf("Weight: %hhu%%\n",rovers[current_rover].weight);
}

int get_option(void)
{
  int var;
  int opt;
  puts("1. Choose rover");
  puts("2. Send cmd to rover");
  puts("3. Execute cmd on rover");
  printf("Option: ");
  var = scanf("%u",&opt);
  if (var != 1)
    die("err");
  return opt;
}


void opt_choose_rover(void)
{
  int var;
  unsigned int rover;
  puts("[Rover list]");
  puts("========================");
  for (int i = 0; i < 0xf; i = i + 1) {
    printf("[%d] %s\n",i,rovers[i].name);
  }
  puts("========================");
  printf("Choose the rover: ");
  var = scanf("%u",&rover);
  if (var != 1)
    die("err");
    
  if (0xe < rover)
    die("Invalid idx");
    
  current_rover = rover;
  puts("Rover selected!");
}


void opt_send_cmd(void)
{
  int var;
  unsigned int action_nmb;
  puts("[Action list]");
  puts("========================");
  for (int i = 0; i < 6; i = i + 1) {
    printf("[%d] %s\n",i,action_names[i]);
  }
  puts("========================");
  printf("Choose the action: ");
  var = scanf("%u",&action_nmb);
  if (var != 1)
    die("err");
  if (5 < action_nmb)
    die("Invalid idx");
      
  printf("Sending command: %s\n",action_names[action_nmb]);
  for (int j = 0; j < 10; j = j + 1) {
    printf(". ");
    usleep(100000);
  }
  puts("");
  rovers[current_rover].code = actions[action_nmb];
  puts("Done!");
}

void opt_execute_cmd(void)
{
  if (rovers[current_rover].code == NULL) {
    puts("Command not selected");
  }
  else {
    puts("Executing command on the rover....");
    rovers[current_rover].code();
    puts("Done!");
  }
}

void init(void)
{
    setvbuf(stdout,NULL,_IONBF,0);
    setvbuf(stdin,NULL,_IONBF,0);
  	setvbuf(stderr,NULL,_IONBF,0);
    // seccomp filter that only allows openat, read, write and clock_nanosleep
    puts("Init done!");
}

void main()
{
  int var;
  unsigned int opt;
  unsigned int joke_size;
  char joke [32];
  init();
  puts("Welcome to the Rover Management System. First of all, I need to verify you\'re an actual  human being. So, please, tell me a funny joke!");
  printf("Joke size: ");
  var = scanf("%u",&joke_size);
  if (var != 1)
    die("err");

  if (0x20 < joke_size)
    die("Invalid joke len");

  printf("Joke: ");
  read_exactly(0,joke,joke_size);
  puts("Hahaha! You\'re fun.");
    
  while(true) {
    opt = get_option();
    switch (opt) {
      case 1:
        opt_choose_rover();
        break;
      case 2:
        opt_send_cmd();
        break;
      case 3:
        opt_execute_cmd();
        break;
      default:
        die("Uknown option");
    }
  }
}
```
