#include "opal_portability.h"

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
  OPAL_PROCESS_CONTROL_POLL_IDLE = 0,
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
  uint64_t last_generation;
};

struct OpalProcessControlPollResult {
  int kind;
  uint64_t generation;
};

static const char *OPAL_ALLOCATION_FAILURE_ERROR = "AllocationFailureError";
static const char *OPAL_SYSTEM_WAIT_SET_ERROR = "SystemWaitSetError";
static const char *OPAL_MONOTONIC_TIMER_ERROR = "MonotonicTimerError";
static const char *OPAL_MONOTONIC_TIMER_NOT_ARMED_ERROR = "MonotonicTimerNotArmedError";
static const char *OPAL_PROCESS_CONTROL_UNAVAILABLE_UNSUPPORTED_HOST_ERROR =
    "ProcessControlUnavailableError.UnsupportedHost";
static const char *OPAL_PROCESS_CONTROL_ERROR = "ProcessControlError";
static const char *OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_ERROR =
    "ProcessControlAcknowledgementError";
static const char *OPAL_PROCESS_CONTROL_RESUME_ERROR = "ProcessControlResumeError";

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
  if (!source) {
    return 0;
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
  source = (OpalProcessControlSource *)calloc(1, sizeof(OpalProcessControlSource));
  if (!source) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  source->last_generation = 0;
  source->readiness_source =
      opal_readiness_source_new(OPAL_READINESS_SOURCE_PROCESS_CONTROL, source);
  if (!source->readiness_source) {
    free(source);
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
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
  OpalProcessControlPollResult *result;
  if (!source) {
    return opal_handle_error(OPAL_PROCESS_CONTROL_ERROR);
  }
  result = (OpalProcessControlPollResult *)calloc(1, sizeof(OpalProcessControlPollResult));
  if (!result) {
    return opal_handle_error(OPAL_ALLOCATION_FAILURE_ERROR);
  }
  result->kind = OPAL_PROCESS_CONTROL_POLL_IDLE;
  result->generation = source->last_generation;
  return opal_handle_success(result);
}

FsVoidResult process_control_acknowledge_suspend(void *source_ptr, uint64_t generation) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  if (!source) {
    return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_ERROR);
  }
  if (generation != 0 && generation != source->last_generation) {
    return opal_void_error(OPAL_PROCESS_CONTROL_ACKNOWLEDGEMENT_ERROR);
  }
  return opal_void_success();
}

FsVoidResult process_control_resume_application(void *source_ptr, uint64_t generation) {
  OpalProcessControlSource *source = (OpalProcessControlSource *)source_ptr;
  if (!source) {
    return opal_void_error(OPAL_PROCESS_CONTROL_RESUME_ERROR);
  }
  if (generation != 0 && generation != source->last_generation) {
    return opal_void_error(OPAL_PROCESS_CONTROL_RESUME_ERROR);
  }
  return opal_void_success();
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
  if (!source) {
    return;
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
