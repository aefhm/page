# Makefile

# Directory for generated files
PUBLIC_DIR = public

# Directory for source files
SRC_DIR = src

# Find all HTML files in the source directory, including nested ones
HTML_FILES = $(shell find $(SRC_DIR) -name '*.html')

# Base template file
BASE_FILE = template.html

# Temporary files
MAIN_TMP = main-section.html
SCRIPT_TMP = script-section.html
TMP_FILE = merged_tmp.html
TIDY_TMP = tidy-tmp.html

# Path to Homebrew-installed Tidy
TIDY = /opt/homebrew/bin/tidy

# Tidy options for older Tidy versions
TIDY_OPTS = \
	--indent auto \
	--indent-spaces 2 \
	--vertical-space yes \
	--wrap 120 \
	--wrap-attributes no \
	--indent-cdata yes \
	--quiet yes \
	--tidy-mark no \
	--force-output yes \
	--show-warnings no \
	--show-errors 0 \
	--doctype html5 \
	--input-encoding utf8 \
    --output-encoding utf8

# Process all HTML files using a for loop
process_all:
	@echo "Processing all HTML files..."
	@for content_file in $(HTML_FILES); do \
		rel_path=$$(echo $$content_file | sed 's|^$(SRC_DIR)/||'); \
		output_file="$(PUBLIC_DIR)/$$rel_path"; \
		output_dir=$$(dirname "$$output_file"); \
		mkdir -p "$$output_dir"; \
		\
		echo "Processing $$content_file -> $$output_file"; \
		\
		echo "Extracting <main> content..."; \
		sed -n '/<main>/,/<\/main>/p' "$$content_file" \
		  | sed '/<main>/d; /<\/main>/d' > $(MAIN_TMP); \
		\
		echo "Extracting <script> content..."; \
		sed -n '/<script type="application\/ld+json">/,/<\/script>/p' "$$content_file" > $(SCRIPT_TMP); \
		\
		echo "Inserting main section after <main>..."; \
		sed '/<main>/r $(MAIN_TMP)' $(BASE_FILE) > $(TMP_FILE); \
		\
		echo "Inserting script before </body>..."; \
		awk '/<\/head>/{system("cat $(SCRIPT_TMP)");print;next}{print}' $(TMP_FILE) > $(TIDY_TMP); \
		\
		echo "Tidying HTML with Homebrew Tidy..."; \
		$(TIDY) $(TIDY_OPTS) -o "$$output_file" $(TIDY_TMP) || cp $(TIDY_TMP) "$$output_file"; \
		\
		echo "Created $$output_file"; \
	done
	@echo "All HTML files processed."
	@echo "Cleaning up temp files..."
	@rm -f $(MAIN_TMP) $(SCRIPT_TMP) $(TMP_FILE) $(TIDY_TMP)

# Clean target to remove generated files
clean:
	rm -f $(MAIN_TMP) $(SCRIPT_TMP) $(TMP_FILE) $(TIDY_TMP)

# Default target
all: process_all

# Phony targets
.PHONY: process_all clean all
