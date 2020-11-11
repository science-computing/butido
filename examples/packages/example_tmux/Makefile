setup: dirs repo
	@echo Ready.

repo_content: repo_dir
	@cp ./pkg.toml /tmp/tmux-repo/pkg.toml

repo: repo_dir repo_content
	cd /tmp/tmux-repo && git init && git add . && git commit -m init

dirs: release_dir staging_dir repo_dir sources_dir

release_dir:
	@mkdir -p /tmp/tmux-releases

staging_dir:
	@mkdir -p /tmp/tmux-staging

repo_dir:
	@mkdir -p /tmp/tmux-repo

sources_dir:
	@mkdir -p /tmp/tmux-sources


dirs_clean: release_dir_clean staging_dir_clean repo_dir_clean sources_dir_clean

release_dir_dir:
	@rm /tmp/tmux-releases/* || true

staging_dir_dir:
	@rm /tmp/tmux-staging/* || true

repo_dir_dir:
	@rm /tmp/tmux-repo/* || true

sources_dir_dir:
	@rm /tmp/tmux-sources/* || true

