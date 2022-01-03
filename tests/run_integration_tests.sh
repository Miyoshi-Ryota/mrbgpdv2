#!/bin/bash
docker-compose -f ./tests/docker-compose.yml build --no-cache
docker-compose -f ./tests/docker-compose.yml up -d
HOST_2_LOOPBACK_IP=10.100.220.3
docker-compose -f ./tests/docker-compose.yml exec -T host1 ping -c 5 $HOST_2_LOOPBACK_IP

# docker-compose execの終了コードは実行したコマンド、
# ここでは`ping -c 5 $HOST_2_LOOPBACK_IP`のものである。
# そのため、BGPでルートを交換し、pingが通れば0, それ以外の場合は1である。
TEST_RESULT=$?
if [ $TEST_RESULT -eq 0 ]; then
    printf "\e[32m%s\e[m\n" "統合テストが成功しました。"
else
    printf "\e[31m%s\e[m\n" "統合テストが失敗しました。"
fi

exit $TEST_RESULT
