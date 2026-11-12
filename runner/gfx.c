#include <bosonoga/bosonoga.h>
#include <bosonoga/gfx.h>
#include <bosonoga/parser.h>
#include <bosonoga/settings.h>
#include <glib.h>
#include <locale.h>

static gboolean corela_bpmn_run_gfx(const gchar *path,
                                    const BosonogaSettings *settings,
                                    gint *exit_status, GError **error) {
  g_autofree gchar *source = NULL;
  gsize source_length = 0;
  if (!g_file_get_contents(path, &source, &source_length, error)) {
    return FALSE;
  }
  g_autofree gchar *display_path = g_filename_display_name(path);
  g_autoptr(BosonogaProgram) program =
      bosonoga_parse(display_path, source, source_length, settings, error);
  if (program == NULL) {
    return FALSE;
  }
  if (!program->is_gfx) {
    g_set_error(error, BOSONOGA_ERROR, BOSONOGA_ERROR_RUNTIME,
                "%s: a gfx app needs gfx_main", display_path);
    return FALSE;
  }
  return bosonoga_gfx_run(program, settings, exit_status, error);
}

int main(int argc, char **argv) {
  setlocale(LC_ALL, "");

  g_autoptr(GError) error = NULL;
  g_autoptr(GOptionContext) context = g_option_context_new("file.bosonoga");
  if (!g_option_context_parse(context, &argc, &argv, &error)) {
    g_printerr("%s\n", error->message);
    return BOSONOGA_EXIT_FAILURE;
  }
  if (argc != 2) {
    g_printerr("usage: %s file.bosonoga\n", g_get_prgname());
    return BOSONOGA_EXIT_FAILURE;
  }

  g_auto(BosonogaSettings) settings = {0};
  if (!bosonoga_read_settings(BOSONOGA_SETTINGS_FILE, &settings, &error)) {
    g_printerr("%s\n", error->message);
    return BOSONOGA_EXIT_FAILURE;
  }
  gint exit_status = BOSONOGA_EXIT_FAILURE;
  if (!corela_bpmn_run_gfx(argv[1], &settings, &exit_status, &error)) {
    g_printerr("%s\n", error->message);
    return BOSONOGA_EXIT_FAILURE;
  }
  return exit_status;
}
