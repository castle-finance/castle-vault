#!/bin/bash
rm -rf test-ledger/

solana-test-validator \
--bpf-program So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo ~/code/solend/solana-program-library/target/deploy/spl_token_lending.so \
--bpf-program Port7uDYB3wk6GJAw4KT1WpTeMtSu9bTcChBHkX2LfR ~/code/port-finance/variable-rate-lending/target/deploy/port_finance_variable_rate_lending.so \
--clone \
    3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E \
    J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix \
    GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR \
--url d