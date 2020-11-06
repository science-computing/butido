REPO=/tmp/butido_example_1

setup: release_dir staging_dir
	@echo Setting up repository in ${REPO}
	@rm -r ${REPO} || true
	@mkdir -p ${REPO}
	@cp -r . ${REPO}/
	@echo Ready.

release_dir:
	@mkdir -p /tmp/butido-releases

staging_dir:
	@mkdir -p /tmp/butido-staging

