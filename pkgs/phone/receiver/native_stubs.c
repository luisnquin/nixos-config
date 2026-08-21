#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  uint8_t *data;
  uintptr_t length;
} xcw_native_owned_bytes;

static char *xcw_strdup(const char *value) {
  if (value == NULL) {
    value = "";
  }
  size_t length = strlen(value);
  char *copy = (char *)malloc(length + 1);
  if (copy != NULL) {
    memcpy(copy, value, length + 1);
  }
  return copy;
}

static bool xcw_unsupported(char **error_message) {
  if (error_message != NULL) {
    *error_message =
        xcw_strdup("iOS simulator native bridge is only available on macOS.");
  }
  return false;
}

static xcw_native_owned_bytes xcw_empty_bytes(char **error_message) {
  xcw_unsupported(error_message);
  xcw_native_owned_bytes bytes = {0};
  return bytes;
}

void xcw_native_initialize_app(void) {}

char *xcw_native_list_simulators(char **error_message) {
  (void)error_message;
  return xcw_strdup("{\"simulators\":[]}");
}

xcw_native_owned_bytes xcw_native_screenshot_png(const char *udid,
                                                 char **error_message) {
  (void)udid;
  return xcw_empty_bytes(error_message);
}

char *xcw_native_accessibility_snapshot(const char *udid, bool has_point,
                                        double x, double y,
                                        uintptr_t max_depth,
                                        bool interactive_only,
                                        char **error_message) {
  (void)udid;
  (void)has_point;
  (void)x;
  (void)y;
  (void)max_depth;
  (void)interactive_only;
  xcw_unsupported(error_message);
  return NULL;
}

bool xcw_native_display_size(const char *udid, double *width_points,
                             double *height_points, double *scale,
                             char **error_message) {
  (void)udid;
  (void)width_points;
  (void)height_points;
  (void)scale;
  return xcw_unsupported(error_message);
}

bool xcw_native_send_key(const char *udid, uint16_t key_code,
                         uint32_t modifiers, char **error_message) {
  (void)udid;
  (void)key_code;
  (void)modifiers;
  return xcw_unsupported(error_message);
}

bool xcw_native_press_button(const char *udid, const char *button_name,
                             uint32_t duration_ms, char **error_message) {
  (void)udid;
  (void)button_name;
  (void)duration_ms;
  return xcw_unsupported(error_message);
}

void *xcw_native_input_create(const char *udid, char **error_message) {
  (void)udid;
  xcw_unsupported(error_message);
  return NULL;
}

void xcw_native_input_destroy(void *handle) { (void)handle; }

bool xcw_native_input_send_touch(void *handle, double x, double y,
                                 const char *phase, char **error_message) {
  (void)handle;
  (void)x;
  (void)y;
  (void)phase;
  return xcw_unsupported(error_message);
}

void xcw_native_free_string(char *value) { free(value); }

void xcw_native_free_bytes(xcw_native_owned_bytes bytes) { free(bytes.data); }
