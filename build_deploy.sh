#!/bin/bash 
# bash -v

set -e

PRJ="rentbot_sslv"
HOST="192.168.0.106"
TARGET="192.168.0.150"

if [ -z "$1" ]
then
      echo "Error: Please provide the ssh password as an argument to the script"
      exit -1
fi


echo "###### Syncing the code base..."
sshpass -p "$1" rsync -av --delete --exclude 'target' --exclude '.git' ./ mimas@$HOST:/home/mimas/prj/$PRJ
if [ $? -ne 0 ]; then
    echo "Error: Fail to RSync data"
    exit 1
fi

    # export PKG_CONFIG_SYSROOT_DIR=~/tools/aarch64-conda-linux-gnu/sysroot &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_DIR=~/tools/aarch64-conda-linux-gnu/sysroot &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR=~/tools/aarch64-conda-linux-gnu/sysroot/usr/include/ &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR=~/tools/aarch64-conda-linux-gnu/sysroot/usr/lib/aarch64-linux-gnu &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_NO_VENDOR=1 &&\

    # export PKG_CONFIG_SYSROOT_DIR=~/tools/pi &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_DIR=~/tools/pi &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR=~/tools/pi/usr/include/ &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR=~/tools/pi/usr/lib/aarch64-linux-gnu &&\
    # export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_NO_VENDOR=1 &&\
echo "###### Building..."
sshpass -p "$1" ssh mimas@$HOST \
    "source  ~/.cargo/env && \
    export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR=~/tools/openssl-3.0.8/include/ &&\
    export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR=~/tools/openssl-3.0.8/ &&\
    export AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_DIR=~/tools/openssl-3.0.8 &&\
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc &&\
    export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc &&\
    export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ && \
    cd ~/prj/$PRJ && cargo build --release --target aarch64-unknown-linux-gnu"
if [ $? -ne 0 ]; then
    echo "Error: Fail to Build the code"
    exit 1
fi

echo "###### Copying to the $TARGET..."
sshpass -p "$1" ssh mimas@$HOST "sshpass -p "$1" rsync -azvP ~/prj/$PRJ/target/aarch64-unknown-linux-gnu/release/$PRJ  mimas@$TARGET:/home/mimas/tmp/$PRJ"
if [ $? -ne 0 ]; then
    echo "Error: Fail to copy the binary to the target"
    exit 1
fi





exit 0




echo "###### Running from the $TARGET..."
sshpass -p "$1" ssh mimas@$TARGET "pkill $PRJ; ~/tmp/$PRJ"
if [ $? -ne 0 ]; then
    echo "Error: To run $PRJ"
    exit 1
fi


