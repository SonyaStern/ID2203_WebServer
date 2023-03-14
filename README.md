Paxos

In this project, you will build a web server that is made fault-tolerant using omnipaxos.
The web server should be accessed via REST or gRPC and store some data that is
accessed in a consistent manner. Furthermore, let each node maintain some statistics on
the number of requests they have handled, and make the node with the most requests
take over leadership when it passes a certain threshold.

1. [In Progress] Fault-tolerant -> Fail recovery tutorial
2. [In Progress] REST for client-server communication -> GET/POST store and access data in a consistent manner
3. [Done] Key/Value
4. Node statistics -> state logs
5. Leader with higher requests, threshold -> leader_priority
6. Testing cases -> https://doc.rust-lang.org/book/ch11-01-writing-tests.html