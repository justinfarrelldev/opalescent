#include "opal_portability.h"

#if !OPAL_WINDOWS
#include <signal.h>
#endif

#ifndef OPAL_FS_HANDLE_RESULT_DEFINED
typedef struct {
  void *value;
  const char *error;
} FsHandleResult;
#define OPAL_FS_HANDLE_RESULT_DEFINED 1
#endif

#ifndef OPAL_FS_VOID_RESULT_TYPE_DEFINED
typedef struct {
  void *value;
  const char *error;
} FsVoidResult;
#define OPAL_FS_VOID_RESULT_TYPE_DEFINED 1
#endif

#ifndef OPAL_PARSE_RESULT_U64_DEFINED
typedef struct {
  uint64_t value;
  const char *error;
} ParseResultU64;
#define OPAL_PARSE_RESULT_U64_DEFINED 1
#endif

typedef struct OpalCancellationSource OpalCancellationSource;
typedef struct OpalCancellationToken OpalCancellationToken;
typedef struct OpalMonotonicDeadline OpalMonotonicDeadline;
typedef struct OpalMonotonicTimer OpalMonotonicTimer;
typedef struct OpalSystemReadinessSource OpalSystemReadinessSource;
typedef struct OpalSystemWaitSet OpalSystemWaitSet;
typedef struct OpalSystemWaitRegistration OpalSystemWaitRegistration;
typedef struct OpalSystemOwnedWaitRegistration OpalSystemOwnedWaitRegistration;
typedef struct OpalSystemWaitWake OpalSystemWaitWake;
typedef struct OpalProcessControlSource OpalProcessControlSource;
typedef struct OpalProcessControlPollResult OpalProcessControlPollResult;

enum {
  OPAL_READINESS_SOURCE_TIMER = 1,
  OPAL_READINESS_SOURCE_PROCESS_CONTROL = 2,
};

enum {
  OPAL_WAIT_WAKE_CANCELLED = 1,
  OPAL_WAIT_WAKE_READY = 2,
};

enum {
  OPAL_PROCESS_CONTROL_NOTIFICATION_SUSPEND_REQUESTED = 0,
  OPAL_PROCESS_CONTROL_NOTIFICATION_CONTINUED = 1,
};

enum {
  OPAL_PROCESS_CONTROL_POLL_NOTIFICATION = 0,
  OPAL_PROCESS_CONTROL_POLL_IDLE = 1,
};

enum {
  OPAL_PROCESS_CONTROL_OBSERVATION_NONE = 0,
  OPAL_PROCESS_CONTROL_OBSERVATION_SUSPEND_REQUESTED = 1,
  OPAL_PROCESS_CONTROL_OBSERVATION_CONTINUED = 2,
};

enum {
  OPAL_PROCESS_CONTROL_GENERATION_STATE_NONE = 0,
  OPAL_PROCESS_CONTROL_GENERATION_STATE_SUSPEND_PENDING = 1,
  OPAL_PROCESS_CONTROL_GENERATION_STATE_ACKNOWLEDGED_HOST_SUSPENDED = 2,
  OPAL_PROCESS_CONTROL_GENERATION_STATE_CONTINUED_AWAITING_APPLICATION_RESUME = 3,
  OPAL_PROCESS_CONTROL_GENERATION_STATE_COMPLETED = 4,
};

enum {
  OPAL_PROCESS_CONTROL_TRANSLATE_IDLE = 0,
  OPAL_PROCESS_CONTROL_TRANSLATE_NOTIFICATION = 1,
  OPAL_PROCESS_CONTROL_TRANSLATE_GENERATION_EXHAUSTED = 2,
};

struct OpalCancellationSource {
  uint64_t generation;
  int requested;
};

struct OpalCancellationToken {
  OpalCancellationSource *source;
  uint64_t generation;
};

struct OpalMonotonicDeadline {
  int64_t milliseconds;
};

struct OpalSystemReadinessSource {
  int kind;
  void *owner;
  uint64_t generation;
};

struct OpalMonotonicTimer {
  uint64_t generation;
  int armed;
  int64_t deadline_ms;
  OpalSystemReadinessSource *readiness_source;
};

typedef struct {
  uint64_t id;
  OpalSystemReadinessSource *source;
  int active;
} OpalWaitEntry;

typedef struct OpalProcessControlQueueEntry {
  int64_t notification_tag;
  uint64_t generation;
  struct OpalProcessControlQueueEntry *next;
} OpalProcessControlQueueEntry;

typedef struct {
  int64_t tag;
  uint8_t payload[64];
} OpalProcessControlNotificationValue;

struct OpalSystemWaitSet {
  uint64_t next_id;
  size_t count;
  size_t capacity;
  OpalWaitEntry *entries;
};

struct OpalSystemWaitRegistration {
  OpalSystemWaitSet *wait_set;
  uint64_t id;
  int removed;
};

struct OpalSystemOwnedWaitRegistration {
  OpalSystemWaitSet *wait_set;
  uint64_t id;
  int removed;
};

struct OpalSystemWaitWake {
  int kind;
  uint64_t observed_id;
};

struct OpalProcessControlSource {
  OpalSystemReadinessSource *readiness_source;
  uint64_t last_issued_generation;
  uint64_t current_generation;
  uint64_t continued_epoch;
  uint64_t consumed_continued_epoch;
  int generation_state;
  int pending_host_observation_kind;
  OpalProcessControlQueueEntry *queue_head;
  OpalProcessControlQueueEntry *queue_tail;
#if !OPAL_WINDOWS
  int read_fd;
  int write_fd;
  struct sigaction previous_suspend_action;
  struct sigaction previous_continue_action;
#endif
};

struct OpalProcessControlPollResult {
  int64_t tag;
  uint8_t payload[64];
};

#if !OPAL_WINDOWS
static volatile sig_atomic_t OPAL_PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD = -1;
static volatile sig_atomic_t OPAL_PROCESS_CONTROL_SIGNAL_PENDING = 0;
static int OPAL_PROCESS_CONTROL_BACKEND_ACTIVE = 0;
#endif

static const char *OPAL_ALLOCATION_FAILURE_ERROR = "AllocationFailureError";
static const char *OPAL_SYSTEM_WAIT_SET_ERROR = "SystemWaitSetError";
static const char *OPAL_MONOTONIC_TIMER_ERROR = "MonotonicTimerError";
static const char *OPAL_MONOTONIC_TIMER_NOT_ARMED_ERROR = "MonotonicTimerNotArmedError";
static const char *OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR =
    "ProcessControlUnavailableError.UnsupportedHost";
static const char *OPAL_PROCESS_CONTROL_ERROR = "ProcessControlError";
static const char *OPAL_PROCESS_CONTROL_ERROR_HOST_NOTIFICATION_OBSERVATION_FAILED =
    "ProcessControlError.HostNotificationObservationFailed";
static const char *OPAL_PROCESS_CONTROL_ERROR_GENERATION_EXHAUSTED =
    "ProcessControlError.GenerationExhausted";
static const char *OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_WRONG_GENERATION_ERROR =
    "ProcessControlAcknowledgementError.WrongGeneration";
static const char *OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_STALE_GENERATION_ERROR =
    "ProcessControlAcknowledgementError.StaleGeneration";
static const char *OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_HOST_SUSPEND_FAILED_ERROR =
    "ProcessControlAcknowledgementError.HostSuspendFailed";
static const char *OPAL_PROCESS_CONTROL_RESUME_WRONG_GENERATION_ERROR =
    "ProcessControlResumeError.WrongGeneration";
static const char *OPAL_PROCESS_CONTROL_RESUME_STALE_GENERATION_ERROR =
    "ProcessControlResumeError.StaleGeneration";
static const char *OPAL_PROCESS_CONTROL_RESUME_HOST_APPLICATION_RESUME_FAILED_ERROR =
    "ProcessControlResumeError.HostApplicationResumeFailed";

void opal_runtime_error(const char *message);

static FsHandleResult opal_handle_success(void *value) {
  FsHandleResult result = {value, NULL};
  return result;
}

static FsHandleResult opal_handle_error(const char *error) {
  FsHandleResult result = {NULL, error};
  return result;
}

static FsVoidResult opal_void_success(void) {
  FsVoidResult result = {NULL, NULL};
  return result;
}

static FsVoidResult opal_void_error(const char *error) {
  FsVoidResult result = {NULL, error};
  return result;
}

static ParseResultU64 opal_u64_success(uint64_t value) {
  ParseResultU64 result = {value, NULL};
  return result;
}

static ParseResultU64 opal_u64_error(const char *error) {
  ParseResultU64 result = {0, error};
  return result;
}

static int opal_next_generation(uint64_t current, uint64_t *next_out) {
  if (!next_out || current == UINT64_MAX) {
    return 0;
  }
  *next_out = current + 1;
  return 1;
}

static OpalSystemReadinessSource *opal_readiness_source_new(int kind, void *owner) {
  OpalSystemReadinessSource *source =
      (OpalSystemReadinessSource *)malloc(sizeof(OpalSystemReadinessSource));
  if (!source) {
    return NULL;
  }
  source->kind = kind;
  source->owner = owner;
  source->generation = 0;
  return source;
}

static void opal_process_control_allocation_failure(void) {
  opal_runtime_error(OPAL_ALLOCATION_FAILURE_ERROR);
}

static void *opal_process_control_calloc_or_abort(size_t count, size_t size) {
  void *value = calloc(count, size);
  if (!value) {
    opal_process_control_allocation_failure();
  }
  return value;
}

static OpalProcessControlNotificationValue *opal_process_control_notification_value_new(
    int64_t tag,
    uint64_t generation) {
  OpalProcessControlNotificationValue *notification =
      (OpalProcessControlNotificationValue *)opal_process_control_calloc_or_abort(
          1, sizeof(OpalProcessControlNotificationValue));
  notification->tag = tag;
  memcpy(notification->payload, &generation, sizeof(generation));
  return notification;
}

static OpalProcessControlPollResult *opal_process_control_poll_idle_value_new(void) {
  OpalProcessControlPollResult *result =
      (OpalProcessControlPollResult *)opal_process_control_calloc_or_abort(
          1, sizeof(OpalProcessControlPollResult));
  result->tag = OPAL_PROCESS_CONTROL_POLL_IDLE;
  return result;
}

static OpalProcessControlPollResult *opal_process_control_poll_notification_value_new(
    int64_t notification_tag,
    uint64_t generation) {
  OpalProcessControlNotificationValue *notification =
      opal_process_control_notification_value_new(notification_tag, generation);
  OpalProcessControlPollResult *result =
      (OpalProcessControlPollResult *)opal_process_control_calloc_or_abort(
          1, sizeof(OpalProcessControlPollResult));
  result->tag = OPAL_PROCESS_CONTROL_POLL_NOTIFICATION;
  memcpy(result->payload, &notification, sizeof(notification));
  return result;
}

static int opal_process_control_has_ready_work(const OpalProcessControlSource *source) {
  if (!source) {
    return 0;
  }
  if (source->queue_head ||
      source->pending_host_observation_kind != OPAL_PROCESS_CONTROL_OBSERVATION_NONE) {
    return 1;
  }
#if !OPAL_WINDOWS
  return OPAL_PROCESS_CONTROL_SIGNAL_PENDING != 0;
#else
  return 0;
#endif
}

#if !OPAL_WINDOWS
static void opal_process_control_close_fd(int fd) {
  if (fd < 0) {
    return;
  }
  while (close(fd) != 0 && errno == EINTR) {
  }
}

static int opal_process_control_configure_nonblocking(int fd) {
  int flags = fcntl(fd, F_GETFL, 0);
  if (flags < 0) {
    return 0;
  }
  return fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0;
}

static int opal_process_control_configure_close_on_exec(int fd) {
  int flags = fcntl(fd, F_GETFD, 0);
  if (flags < 0) {
    return 0;
  }
  return fcntl(fd, F_SETFD, flags | FD_CLOEXEC) == 0;
}

static int opal_process_control_create_signal_pipe(int *read_fd_out, int *write_fd_out) {
  int pipe_fds[2];
  if (!read_fd_out || !write_fd_out) {
    return 0;
  }
  if (pipe(pipe_fds) != 0) {
    return 0;
  }
  if (!opal_process_control_configure_nonblocking(pipe_fds[0]) ||
      !opal_process_control_configure_nonblocking(pipe_fds[1]) ||
      !opal_process_control_configure_close_on_exec(pipe_fds[0]) ||
      !opal_process_control_configure_close_on_exec(pipe_fds[1])) {
    opal_process_control_close_fd(pipe_fds[0]);
    opal_process_control_close_fd(pipe_fds[1]);
    return 0;
  }
  *read_fd_out = pipe_fds[0];
  *write_fd_out = pipe_fds[1];
  return 1;
}

static void process_control_signal_handler(int signal_number) {
  sig_atomic_t write_fd = OPAL_PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD;
  uint8_t signal_code;
  if (signal_number == SIGTSTP) {
    signal_code = OPAL_PROCESS_CONTROL_OBSERVATION_SUSPEND_REQUESTED;
  } else if (signal_number == SIGCONT) {
    signal_code = OPAL_PROCESS_CONTROL_OBSERVATION_CONTINUED;
  } else {
    return;
  }
  OPAL_PROCESS_CONTROL_SIGNAL_PENDING = 1;
  if (write_fd < 0) {
    return;
  }
  (void)write((int)write_fd, &signal_code, 1);
}

static void opal_process_control_zero_sigaction(struct sigaction *action) {
  if (!action) {
    return;
  }
  memset(action, 0, sizeof(*action));
}

static int opal_process_control_install_signal_handler(
    int signal_number,
    struct sigaction *previous_action) {
  struct sigaction action;
  opal_process_control_zero_sigaction(&action);
  action.sa_handler = process_control_signal_handler;
  sigemptyset(&action.sa_mask);
  action.sa_flags = SA_RESTART;
  return sigaction(signal_number, &action, previous_action) == 0;
}

static void opal_process_control_restore_signal_handler(
    int signal_number,
    const struct sigaction *previous_action) {
  if (!previous_action) {
    return;
  }
  (void)sigaction(signal_number, previous_action, NULL);
}

static int opal_process_control_observe_notification(
    OpalProcessControlSource *source,
    int *observation_kind_out) {
  uint8_t signal_code;
  ssize_t read_result;
  if (!source || !observation_kind_out) {
    return -1;
  }
  for (;;) {
    read_result = read(source->read_fd, &signal_code, 1);
    if (read_result == 0) {
      return -1;
    }
    if (read_result < 0) {
      if (errno == EINTR) {
        continue;
      }
      if (errno == EAGAIN || errno == EWOULDBLOCK) {
        if (!source->queue_head &&
            source->pending_host_observation_kind == OPAL_PROCESS_CONTROL_OBSERVATION_NONE) {
          OPAL_PROCESS_CONTROL_SIGNAL_PENDING = 0;
        }
        return 0;
      }
      return -1;
    }
    OPAL_PROCESS_CONTROL_SIGNAL_PENDING = 1;
    if (signal_code == OPAL_PROCESS_CONTROL_OBSERVATION_SUSPEND_REQUESTED) {
      *observation_kind_out = OPAL_PROCESS_CONTROL_OBSERVATION_SUSPEND_REQUESTED;
      return 1;
    }
    if (signal_code == OPAL_PROCESS_CONTROL_OBSERVATION_CONTINUED) {
      *observation_kind_out = OPAL_PROCESS_CONTROL_OBSERVATION_CONTINUED;
      return 1;
    }
  }
}
#endif

static int opal_process_control_enqueue_notification(
    OpalProcessControlSource *source,
    int64_t notification_tag,
    uint64_t generation) {
  OpalProcessControlQueueEntry *entry;
  if (!source) {
    return 0;
  }
  entry = (OpalProcessControlQueueEntry *)opal_process_control_calloc_or_abort(
      1, sizeof(OpalProcessControlQueueEntry));
  entry->notification_tag = notification_tag;
  entry->generation = generation;
  if (source->queue_tail) {
    source->queue_tail->next = entry;
  } else {
    source->queue_head = entry;
  }
  source->queue_tail = entry;
  return 1;
}

static int opal_process_control_dequeue_notification(
    OpalProcessControlSource *source,
    int64_t *notification_tag_out,
    uint64_t *generation_out) {
  OpalProcessControlQueueEntry *entry;
  if (!source || !source->queue_head || !notification_tag_out || !generation_out) {
    return 0;
  }
  entry = source->queue_head;
  source->queue_head = entry->next;
  if (!source->queue_head) {
    source->queue_tail = NULL;
  }
  *notification_tag_out = entry->notification_tag;
  *generation_out = entry->generation;
  free(entry);
  return 1;
}

static int opal_process_control_translate_pending_observation(
    OpalProcessControlSource *source,
    int64_t *notification_tag_out,
    uint64_t *generation_out) {
  uint64_t next_generation;
  if (!source || !notification_tag_out || !generation_out) {
    return OPAL_PROCESS_CONTROL_TRANSLATE_IDLE;
  }

  if (source->pending_host_observation_kind == OPAL_PROCESS_CONTROL_OBSERVATION_SUSPEND_REQUESTED) {
    if (source->generation_state == OPAL_PROCESS_CONTROL_GENERATION_STATE_NONE ||
        source->generation_state == OPAL_PROCESS_CONTROL_GENERATION_STATE_COMPLETED) {
      if (!opal_next_generation(source->last_issued_generation, &next_generation)) {
        return OPAL_PROCESS_CONTROL_TRANSLATE_GENERATION_EXHAUSTED;
      }
      source->last_issued_generation = next_generation;
      source->current_generation = next_generation;
      source->generation_state = OPAL_PROCESS_CONTROL_GENERATION_STATE_SUSPEND_PENDING;
      *notification_tag_out = OPAL_PROCESS_CONTROL_NOTIFICATION_SUSPEND_REQUESTED;
      *generation_out = next_generation;
      return OPAL_PROCESS_CONTROL_TRANSLATE_NOTIFICATION;
    }
    if (source->generation_state == OPAL_PROCESS_CONTROL_GENERATION_STATE_SUSPEND_PENDING &&
        source->current_generation != 0) {
      *notification_tag_out = OPAL_PROCESS_CONTROL_NOTIFICATION_SUSPEND_REQUESTED;
      *generation_out = source->current_generation;
      return OPAL_PROCESS_CONTROL_TRANSLATE_NOTIFICATION;
    }
    return OPAL_PROCESS_CONTROL_TRANSLATE_IDLE;
  }

  if (source->pending_host_observation_kind == OPAL_PROCESS_CONTROL_OBSERVATION_CONTINUED) {
    if (source->continued_epoch != UINT64_MAX) {
      source->continued_epoch += 1;
    }
    if (source->generation_state ==
            OPAL_PROCESS_CONTROL_GENERATION_STATE_ACKNOWLEDGED_HOST_SUSPENDED &&
        source->current_generation != 0) {
      source->generation_state =
          OPAL_PROCESS_CONTROL_GENERATION_STATE_CONTINUED_AWAITING_APPLICATION_RESUME;
      *notification_tag_out = OPAL_PROCESS_CONTROL_NOTIFICATION_CONTINUED;
      *generation_out = source->current_generation;
      return OPAL_PROCESS_CONTROL_TRANSLATE_NOTIFICATION;
    }
  }

  return OPAL_PROCESS_CONTROL_TRANSLATE_IDLE;
}

static int opal_wait_set_ensure_capacity(OpalSystemWaitSet *wait_set) {
  size_t new_capacity;
  OpalWaitEntry *new_entries;
  if (!wait_set) {
    return 0;
  }
  if (wait_set->count < wait_set->capacity) {
    return 1;
  }
  new_capacity = wait_set->capacity == 0 ? 4 : wait_set->capacity * 2;
  new_entries = (OpalWaitEntry *)realloc(wait_set->entries, new_capacity * sizeof(OpalWaitEntry));
  if (!new_entries) {
    return 0;
  }
  wait_set->entries = new_entries;
  wait_set->capacity = new_capacity;
  return 1;
}

static OpalWaitEntry *opal_wait_set_find_entry(OpalSystemWaitSet *wait_set, uint64_t id) {
  size_t index;
  if (!wait_set) {
    return NULL;
  }
  for (index = 0; index < wait_set->count; index++) {
    if (wait_set->entries[index].id == id) {
      return &wait_set->entries[index];
    }
  }
  return NULL;
}

static int opal_readiness_source_is_ready(OpalSystemReadinessSource *source) {
  int64_t now_ms;
  OpalMonotonicTimer *timer;
  OpalProcessControlSource *process_source;
  if (!source) {
    return 0;
  }
  if (source->kind == OPAL_READINESS_SOURCE_PROCESS_CONTROL) {
    process_source = (OpalProcessControlSource *)source->owner;
    return opal_process_control_has_ready_work(process_source);
  }
  if (source->kind != OPAL_READINESS_SOURCE_TIMER) {
    return 0;
  }
  timer = (OpalMonotonicTimer *)source->owner;
  if (!timer || !timer->armed) {
    return 0;
  }
  if (opal_monotonic_time_ms(&now_ms) != 0) {
    return 0;
  }
  return now_ms >= timer->deadline_ms;
}

FsHandleResult system_wait_set_new(void) {
  OpalSystemWaitSet *wait_set = (OpalSystemWaitSet *)calloc(1, sizeof(OpalSystemWaitSet));
  if (!wait_set) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  wait_set->next_id = 1;
  return opal_handle_success(wait_set);
}

FsHandleResult system_wait_set_register(void *wait_set_ptr, void *source_ptr) {
  OpalSystemWaitSet *wait_set = (OpalSystemWaitSet *)wait_set_ptr;
  OpalSystemReadinessSource *source = (OpalSystemReadinessSource *)source_ptr;
  OpalSystemWaitRegistration *registration;
  OpalWaitEntry *entry;
  if (!wait_set || !source) {
    return opal_handle_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  if (!opal_wait_set_ensure_capacity(wait_set)) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  registration = (OpalSystemWaitRegistration *)calloc(1, sizeof(OpalSystemWaitRegistration));
  if (!registration) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  entry = &wait_set->entries[wait_set->count++];
  entry->id = wait_set->next_id++;
  entry->source = source;
  entry->active = 1;
  registration->wait_set = wait_set;
  registration->id = entry->id;
  registration->removed = 0;
  return opal_handle_success(registration);
}

FsVoidResult system_wait_set_remove(void *wait_set_ptr, void *registration_ptr) {
  OpalSystemWaitSet *wait_set = (OpalSystemWaitSet *)wait_set_ptr;
  OpalSystemWaitRegistration *registration =
      (OpalSystemWaitRegistration *)registration_ptr;
  OpalWaitEntry *entry;
  if (!wait_set || !registration || registration->wait_set != wait_set) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  if (registration->removed) {
    return opal_void_success();
  }
  entry = opal_wait_set_find_entry(wait_set, registration->id);
  if (!entry) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  entry->active = 0;
  entry->source = NULL;
  registration->removed = 1;
  return opal_void_success();
}

FsHandleResult system_wait_set_register_owned(void *wait_set_ptr, void *source_ptr) {
  OpalSystemWaitSet *wait_set = (OpalSystemWaitSet *)wait_set_ptr;
  OpalSystemReadinessSource *source = (OpalSystemReadinessSource *)source_ptr;
  OpalSystemOwnedWaitRegistration *registration;
  OpalWaitEntry *entry;
  if (!wait_set || !source) {
    return opal_handle_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  if (!opal_wait_set_ensure_capacity(wait_set)) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  registration =
      (OpalSystemOwnedWaitRegistration *)calloc(1, sizeof(OpalSystemOwnedWaitRegistration));
  if (!registration) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  entry = &wait_set->entries[wait_set->count++];
  entry->id = wait_set->next_id++;
  entry->source = source;
  entry->active = 1;
  registration->wait_set = wait_set;
  registration->id = entry->id;
  registration->removed = 0;
  return opal_handle_success(registration);
}

FsVoidResult system_owned_wait_registration_retarget(void *registration_ptr, void *source_ptr) {
  OpalSystemOwnedWaitRegistration *registration =
      (OpalSystemOwnedWaitRegistration *)registration_ptr;
  OpalSystemReadinessSource *source = (OpalSystemReadinessSource *)source_ptr;
  OpalWaitEntry *entry;
  if (!registration || !registration->wait_set || !source) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  if (registration->removed) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  entry = opal_wait_set_find_entry(registration->wait_set, registration->id);
  if (!entry || !entry->active) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  entry->source = source;
  return opal_void_success();
}

FsVoidResult system_owned_wait_registration_remove(void *registration_ptr) {
  OpalSystemOwnedWaitRegistration *registration =
      (OpalSystemOwnedWaitRegistration *)registration_ptr;
  OpalWaitEntry *entry;
  if (!registration || !registration->wait_set) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  if (registration->removed) {
    return opal_void_success();
  }
  entry = opal_wait_set_find_entry(registration->wait_set, registration->id);
  if (!entry) {
    return opal_void_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  entry->active = 0;
  entry->source = NULL;
  registration->removed = 1;
  return opal_void_success();
}

FsHandleResult system_wait_set_wait_sync(void *wait_set_ptr, void *cancellation_ptr) {
  OpalSystemWaitSet *wait_set = (OpalSystemWaitSet *)wait_set_ptr;
  OpalCancellationToken *token = (OpalCancellationToken *)cancellation_ptr;
  size_t index;
  if (!wait_set || !token || !token->source) {
    return opal_handle_error(OPAL_SYSTEM_WAIT_SET_ERROR);
  }
  for (;;) {
    OpalSystemWaitWake *wake;
    if (token->source->requested && token->generation == token->source->generation) {
      wake = (OpalSystemWaitWake *)calloc(1, sizeof(OpalSystemWaitWake));
      if (!wake) {
        return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
      }
      wake->kind = OPAL_WAIT_WAKE_CANCELLED;
      return opal_handle_success(wake);
    }
    for (index = 0; index < wait_set->count; index++) {
      if (wait_set->entries[index].active &&
          opal_readiness_source_is_ready(wait_set->entries[index].source)) {
        wake = (OpalSystemWaitWake *)calloc(1, sizeof(OpalSystemWaitWake));
        if (!wake) {
          return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
        }
        wake->kind = OPAL_WAIT_WAKE_READY;
        wake->observed_id = wait_set->entries[index].id;
        return opal_handle_success(wake);
      }
    }
    if (wait_set->count == 0) {
      return opal_handle_error(OPAL_SYSTEM_WAIT_SET_ERROR);
    }
    opal_sleep_ms(1);
  }
}

FsHandleResult cancellation_source_new(void) {
  OpalCancellationSource *source =
      (OpalCancellationSource *)calloc(1, sizeof(OpalCancellationSource));
  if (!source) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  source->generation = 1;
  source->requested = 0;
  return opal_handle_success(source);
}

void *cancellation_token(void *source_ptr) {
  OpalCancellationSource *source = (OpalCancellationSource *)source_ptr;
  OpalCancellationToken *token;
  if (!source) {
    opal_runtime_error(OPAL_SYSTEM_WAIT_SET_ERROR);
    return NULL;
  }
  token = (OpalCancellationToken *)calloc(1, sizeof(OpalCancellationToken));
  if (!token) {
    opal_runtime_error(OPAL_ALLOCATION_FAILURE_ERROR);
    return NULL;
  }
  token->source = source;
  token->generation = source->generation;
  return token;
}

void cancellation_request(void *source_ptr) {
  OpalCancellationSource *source = (OpalCancellationSource *)source_ptr;
  if (!source) {
    return;
  }
  source->requested = 1;
}

FsHandleResult monotonic_timer_new(void) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)calloc(1, sizeof(OpalMonotonicTimer));
  if (!timer) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  timer->generation = 1;
  timer->armed = 0;
  timer->deadline_ms = 0;
  timer->readiness_source = opal_readiness_source_new(OPAL_READINESS_SOURCE_TIMER, timer);
  if (!timer->readiness_source) {
    free(timer);
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  return opal_handle_success(timer);
}

void *monotonic_timer_readiness_source(void *timer_ptr) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)timer_ptr;
  if (!timer || !timer->readiness_source) {
    opal_runtime_error(OPAL_MONOTONIC_TIMER_ERROR);
    return NULL;
  }
  return timer->readiness_source;
}

ParseResultU64 monotonic_timer_arm(void *timer_ptr, void *deadline_ptr) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)timer_ptr;
  OpalMonotonicDeadline *deadline = (OpalMonotonicDeadline *)deadline_ptr;
  uint64_t next_generation;
  if (!timer || !deadline) {
    return opal_u64_error(OPAL_MONOTONIC_TIMER_ERROR);
  }
  if (!opal_next_generation(timer->generation, &next_generation)) {
    return opal_u64_error(OPAL_MONOTONIC_TIMER_ERROR);
  }
  timer->generation = next_generation;
  timer->armed = 1;
  timer->deadline_ms = deadline->milliseconds;
  timer->readiness_source->generation = next_generation;
  return opal_u64_success(next_generation);
}

ParseResultU64 monotonic_timer_disarm(void *timer_ptr) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)timer_ptr;
  uint64_t next_generation;
  if (!timer) {
    return opal_u64_error(OPAL_MONOTONIC_TIMER_ERROR);
  }
  if (!opal_next_generation(timer->generation, &next_generation)) {
    return opal_u64_error(OPAL_MONOTONIC_TIMER_ERROR);
  }
  timer->generation = next_generation;
  timer->armed = 0;
  timer->deadline_ms = 0;
  timer->readiness_source->generation = next_generation;
  return opal_u64_success(next_generation);
}

uint64_t monotonic_timer_generation(void *timer_ptr) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)timer_ptr;
  if (!timer) {
    opal_runtime_error(OPAL_MONOTONIC_TIMER_ERROR);
    return 0;
  }
  return timer->generation;
}

FsHandleResult monotonic_timer_deadline(void *timer_ptr) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)timer_ptr;
  OpalMonotonicDeadline *deadline;
  if (!timer || !timer->armed) {
    return opal_handle_error(OPAL_MONOTONIC_TIMER_NOT_ARMED_ERROR);
  }
  deadline = (OpalMonotonicDeadline *)calloc(1, sizeof(OpalMonotonicDeadline));
  if (!deadline) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  deadline->milliseconds = timer->deadline_ms;
  return opal_handle_success(deadline);
}

void *monotonic_clock_now(void) {
  int64_t now_ms;
  OpalMonotonicDeadline *deadline;
  if (opal_monotonic_time_ms(&now_ms) != 0) {
    opal_runtime_error(OPAL_MONOTONIC_TIMER_ERROR);
    return NULL;
  }
  deadline = (OpalMonotonicDeadline *)calloc(1, sizeof(OpalMonotonicDeadline));
  if (!deadline) {
    opal_runtime_error(OPAL_ALLOCATION_FAILURE_ERROR);
    return NULL;
  }
  deadline->milliseconds = now_ms;
  return deadline;
}

FsHandleResult process_control_source_new(void) {
  OpalProcessControlSource *source;
#if OPAL_WINDOWS
  return opal_handle_error(OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR);
#else
  int read_fd = -1;
  int write_fd = -1;
  struct sigaction previous_suspend_action;
  struct sigaction previous_continue_action;
  if (OPAL_PROCESS_CONTROL_BACKEND_ACTIVE) {
    return opal_handle_error(OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR);
  }
  if (!opal_process_control_create_signal_pipe(&read_fd, &write_fd)) {
    return opal_handle_error(OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR);
  }
  opal_process_control_zero_sigaction(&previous_suspend_action);
  opal_process_control_zero_sigaction(&previous_continue_action);
  if (!opal_process_control_install_signal_handler(SIGTSTP, &previous_suspend_action)) {
    opal_process_control_close_fd(write_fd);
    opal_process_control_close_fd(read_fd);
    return opal_handle_error(OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR);
  }
  if (!opal_process_control_install_signal_handler(SIGCONT, &previous_continue_action)) {
    opal_process_control_restore_signal_handler(SIGTSTP, &previous_suspend_action);
    opal_process_control_close_fd(write_fd);
    opal_process_control_close_fd(read_fd);
    return opal_handle_error(OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR);
  }
  source = (OpalProcessControlSource *)calloc(1, sizeof(OpalProcessControlSource));
  if (!source) {
    opal_process_control_restore_signal_handler(SIGCONT, &previous_continue_action);
    opal_process_control_restore_signal_handler(SIGTSTP, &previous_suspend_action);
    opal_process_control_close_fd(write_fd);
    opal_process_control_close_fd(read_fd);
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  source->readiness_source =
      opal_readiness_source_new(OPAL_READINESS_SOURCE_PROCESS_CONTROL, source);
  if (!source->readiness_source) {
    free(source);
    opal_process_control_restore_signal_handler(SIGCONT, &previous_continue_action);
    opal_process_control_restore_signal_handler(SIGTSTP, &previous_suspend_action);
    opal_process_control_close_fd(write_fd);
    opal_process_control_close_fd(read_fd);
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  source->last_issued_generation = 0;
  source->current_generation = 0;
  source->continued_epoch = 0;
  source->consumed_continued_epoch = 0;
  source->generation_state = OPAL_PROCESS_CONTROL_GENERATION_STATE_NONE;
  source->pending_host_observation_kind = OPAL_PROCESS_CONTROL_OBSERVATION_NONE;
  source->queue_head = NULL;
  source->queue_tail = NULL;
  source->read_fd = read_fd;
  source->write_fd = write_fd;
  source->previous_suspend_action = previous_suspend_action;
  source->previous_continue_action = previous_continue_action;
  OPAL_PROCESS_CONTROL_SIGNAL_PENDING = 0;
  OPAL_PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD = write_fd;
  OPAL_PROCESS_CONTROL_BACKEND_ACTIVE = 1;
  return opal_handle_success(source);
#endif
}

void *process_control_readiness_source(void *source_ptr) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  if (!source || !source->readiness_source) {
    opal_runtime_error(OPAL_PROCESS_CONTROL_ERROR);
    return NULL;
  }
  return source->readiness_source;
}

FsHandleResult process_control_poll(void *source_ptr) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  int64_t notification_tag;
  uint64_t generation;
  int translation_status;
  if (!source) {
    return opal_handle_error(OPAL_PROCESS_CONTROL_ERROR_HOST_NOTIFICATION_OBSERVATION_FAILED);
  }
  if (opal_process_control_dequeue_notification(source, &notification_tag, &generation)) {
    return opal_handle_success(
        opal_process_control_poll_notification_value_new(notification_tag, generation));
  }
  if (source->pending_host_observation_kind == OPAL_PROCESS_CONTROL_OBSERVATION_NONE) {
#if OPAL_WINDOWS
    return opal_handle_success(opal_process_control_poll_idle_value_new());
#else
    int observation_kind = OPAL_PROCESS_CONTROL_OBSERVATION_NONE;
    int observation_status =
        opal_process_control_observe_notification(source, &observation_kind);
    if (observation_status < 0) {
      return opal_handle_error(OPAL_PROCESS_CONTROL_ERROR_HOST_NOTIFICATION_OBSERVATION_FAILED);
    }
    if (observation_status == 0) {
      return opal_handle_success(opal_process_control_poll_idle_value_new());
    }
    source->pending_host_observation_kind = observation_kind;
#endif
  }
  translation_status = opal_process_control_translate_pending_observation(
      source, &notification_tag, &generation);
  if (translation_status == OPAL_PROCESS_CONTROL_TRANSLATE_GENERATION_EXHAUSTED) {
    return opal_handle_error(OPAL_PROCESS_CONTROL_ERROR_GENERATION_EXHAUSTED);
  }
  source->pending_host_observation_kind = OPAL_PROCESS_CONTROL_OBSERVATION_NONE;
  if (translation_status != OPAL_PROCESS_CONTROL_TRANSLATE_NOTIFICATION) {
    return opal_handle_success(opal_process_control_poll_idle_value_new());
  }
  opal_process_control_enqueue_notification(source, notification_tag, generation);
  if (!opal_process_control_dequeue_notification(source, &notification_tag, &generation)) {
    return opal_handle_success(opal_process_control_poll_idle_value_new());
  }
  return opal_handle_success(
      opal_process_control_poll_notification_value_new(notification_tag, generation));
}

FsVoidResult process_control_acknowledge_suspend(void *source_ptr, uint64_t generation) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  if (!source || source->current_generation == 0) {
    return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_WRONG_GENERATION_ERROR);
  }
  if (source->current_generation != generation) {
    return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_WRONG_GENERATION_ERROR);
  }
  if (source->generation_state == OPAL_PROCESS_CONTROL_GENERATION_STATE_SUSPEND_PENDING) {
#if OPAL_WINDOWS
    return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_HOST_SUSPEND_FAILED_ERROR);
#else
    pid_t process_id = getpid();
    if (kill(process_id, SIGSTOP) != 0) {
      return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_HOST_SUSPEND_FAILED_ERROR);
    }
    source->generation_state =
        OPAL_PROCESS_CONTROL_GENERATION_STATE_ACKNOWLEDGED_HOST_SUSPENDED;
    return opal_void_success();
#endif
  }
  if (source->generation_state ==
      OPAL_PROCESS_CONTROL_GENERATION_STATE_ACKNOWLEDGED_HOST_SUSPENDED) {
    return opal_void_success();
  }
  return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_STALE_GENERATION_ERROR);
}

FsVoidResult process_control_resume_application(void *source_ptr, uint64_t generation) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  if (!source || source->current_generation == 0) {
    return opal_void_error(OPAL_PROCESS_CONTROL_RESUME_WRONG_GENERATION_ERROR);
  }
  if (source->current_generation != generation) {
    return opal_void_error(OPAL_PROCESS_CONTROL_RESUME_WRONG_GENERATION_ERROR);
  }
  if (source->generation_state ==
      OPAL_PROCESS_CONTROL_GENERATION_STATE_CONTINUED_AWAITING_APPLICATION_RESUME) {
    if (source->continued_epoch == source->consumed_continued_epoch) {
      return opal_void_error(
          OPAL_PROCESS_CONTROL_RESUME_HOST_APPLICATION_RESUME_FAILED_ERROR);
    }
    source->consumed_continued_epoch = source->continued_epoch;
    source->generation_state = OPAL_PROCESS_CONTROL_GENERATION_STATE_COMPLETED;
    return opal_void_success();
  }
  if (source->generation_state == OPAL_PROCESS_CONTROL_GENERATION_STATE_COMPLETED) {
    return opal_void_success();
  }
  return opal_void_error(OPAL_PROCESS_CONTROL_RESUME_STALE_GENERATION_ERROR);
}

void system_wait_set_drop(void *wait_set_ptr) {
  OpalSystemWaitSet *wait_set = (OpalSystemWaitSet *)wait_set_ptr;
  if (!wait_set) {
    return;
  }
  free(wait_set->entries);
  free(wait_set);
}

void process_control_source_drop(void *source_ptr) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  OpalProcessControlQueueEntry *entry;
  if (!source) {
    return;
  }
#if !OPAL_WINDOWS
  opal_process_control_restore_signal_handler(SIGCONT, &source->previous_continue_action);
  opal_process_control_restore_signal_handler(SIGTSTP, &source->previous_suspend_action);
  OPAL_PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD = -1;
  OPAL_PROCESS_CONTROL_SIGNAL_PENDING = 0;
  opal_process_control_close_fd(source->write_fd);
  opal_process_control_close_fd(source->read_fd);
  OPAL_PROCESS_CONTROL_BACKEND_ACTIVE = 0;
#endif
  entry = source->queue_head;
  while (entry) {
    OpalProcessControlQueueEntry *next = entry->next;
    free(entry);
    entry = next;
  }
  free(source->readiness_source);
  free(source);
}

void monotonic_timer_drop(void *timer_ptr) {
  OpalMonotonicTimer *timer = (OpalMonotonicTimer *)timer_ptr;
  if (!timer) {
    return;
  }
  free(timer->readiness_source);
  free(timer);
}

void cancellation_source_drop(void *source_ptr) {
  OpalCancellationSource *source = (OpalCancellationSource *)source_ptr;
  free(source);
}
