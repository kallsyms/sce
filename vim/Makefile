.PHONY: all
all: protos

.PHONY: protos
protos: ../sce.proto
	python3 -m grpc_tools.protoc -I ../ --python_out=. --grpc_python_out=. sce.proto
