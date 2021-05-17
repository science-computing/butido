export BUTIDO_RELEASES="/tmp/butido-test-releases"
export BUTIDO_STAGING="/tmp/butido-test-staging"
export BUTIDO_SOURCE_CACHE="/tmp/butido-test-sources"
export BUTIDO_LOG_DIR="/tmp/butido-test-logs"
export BUTIDO_REPO="/tmp/butido-test-repo"

.PHONY: all
all: directories copyrepo copysrc

directories: ${BUTIDO_RELEASES} ${BUTIDO_STAGING} ${BUTIDO_SOURCE_CACHE} ${BUTIDO_LOG_DIR} ${BUTIDO_REPO}

copyrepo: ${BUTIDO_REPO}
	cp -rv ./repo/* ${BUTIDO_REPO}/
	cd ${BUTIDO_REPO}/ && git init && git add . && git commit -m init

copysrc: ${BUTIDO_SOURCE_CACHE}
	cp -rv ./sources/* ${BUTIDO_SOURCE_CACHE}/

${BUTIDO_RELEASES}:
	mkdir -p "${BUTIDO_RELEASES}/default"

${BUTIDO_STAGING}:
	mkdir -p "${BUTIDO_STAGING}"

${BUTIDO_SOURCE_CACHE}:
	mkdir -p "${BUTIDO_SOURCE_CACHE}"

${BUTIDO_LOG_DIR}:
	mkdir -p "${BUTIDO_LOG_DIR}"

${BUTIDO_REPO}:
	mkdir -p "${BUTIDO_REPO}"


