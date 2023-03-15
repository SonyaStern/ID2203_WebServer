Paxos

In this project, you will build a web server that is made fault-tolerant using omnipaxos.
The web server should be accessed via REST or gRPC and store some data that is
accessed in a consistent manner. Furthermore, let each node maintain some statistics on
the number of requests they have handled, and make the node with the most requests
take over leadership when it passes a certain threshold.

1. [In Progress] Fault-tolerant -> Fail recovery tutorial
2. [Done] REST for client-server communication -> GET/POST store and 
3. Access data in a consistent manner -> Load tests?
4. [Done] Key/Value
5. Node statistics -> state logs
6. Leader with higher requests, threshold -> leader_priority
7. Testing cases -> https://doc.rust-lang.org/book/ch11-01-writing-tests.html