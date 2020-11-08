(function() {var implementors = {};
implementors["hyper"] = [{"text":"impl AsRawFd for AddrStream","synthetic":false,"types":[]}];
implementors["mio"] = [{"text":"impl AsRawFd for Poll","synthetic":false,"types":[]},{"text":"impl AsRawFd for TcpStream","synthetic":false,"types":[]},{"text":"impl AsRawFd for TcpListener","synthetic":false,"types":[]},{"text":"impl AsRawFd for UdpSocket","synthetic":false,"types":[]}];
implementors["net2"] = [{"text":"impl AsRawFd for TcpBuilder","synthetic":false,"types":[]},{"text":"impl AsRawFd for UdpBuilder","synthetic":false,"types":[]}];
implementors["socket2"] = [{"text":"impl AsRawFd for Socket","synthetic":false,"types":[]}];
implementors["tokio"] = [{"text":"impl AsRawFd for TcpListener","synthetic":false,"types":[]},{"text":"impl AsRawFd for TcpStream","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()