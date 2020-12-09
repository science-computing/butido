.PHONY: all
all: directories copyrepo

directories: ${BUTIDO_RELEASES} ${BUTIDO_STAGING} ${BUTIDO_SOURCE_CACHE} ${BUTIDO_LOG_DIR} ${BUTIDO_REPO}


# to start a webserver for serving the source files.
webserver: ./sources ${BUTIDO_SOURCE_CACHE}
	devd -A 127.0.0.1 -p 8180 -T ./sources


copyrepo: ${BUTIDO_REPO} check-env
	cp -rv ./repo/* ${BUTIDO_REPO}/
	cd ${BUTIDO_REPO}/ && git init && git add . && git commit -m init

${BUTIDO_RELEASES}: check-env
	mkdir -p "${BUTIDO_RELEASES}"

${BUTIDO_STAGING}: check-env
	mkdir -p "${BUTIDO_STAGING}"

${BUTIDO_SOURCE_CACHE}: check-env
	mkdir -p "${BUTIDO_SOURCE_CACHE}"

${BUTIDO_LOG_DIR}: check-env
	mkdir -p "${BUTIDO_LOG_DIR}"

${BUTIDO_REPO}: check-env
	mkdir -p "${BUTIDO_REPO}"


check-env:
ifndef BUTIDO_RELEASES
	$(error BUTIDO_RELEASES is undefined)
endif
ifndef BUTIDO_STAGING
	$(error BUTIDO_STAGING is undefined)
endif
ifndef BUTIDO_SOURCE_CACHE
	$(error BUTIDO_SOURCE_CACHE is undefined)
endif
ifndef BUTIDO_LOG_DIR
	$(error BUTIDO_LOG_DIR is undefined)
endif
ifndef BUTIDO_REPO
	$(error BUTIDO_REPO is undefined)
endif
ifndef BUTIDO_DATABASE_HOST
	$(error BUTIDO_DATABASE_HOST is undefined)
endif
ifndef BUTIDO_DATABASE_PORT
	$(error BUTIDO_DATABASE_PORT is undefined)
endif
ifndef BUTIDO_DATABASE_USER
	$(error BUTIDO_DATABASE_USER is undefined)
endif
ifndef BUTIDO_DATABASE_PASSWORD
	$(error BUTIDO_DATABASE_PASSWORD is undefined)
endif
ifndef BUTIDO_DATABASE_NAME
	$(error BUTIDO_DATABASE_NAME is undefined)
endif
