# How to run this

1. Start the slippy map caching server. In `slippy-map/tile-cache`, run `cargo run --features online`. This will download new tiles as needed, which may be slow initially, so make sure to zoom around the area of interest beforehand. The server is listening at `localhost:3000`.
2. Start the backend. In `backend`, run `make`. The server is set to IP address `10.69.69.3` and is listening on port `8080`, which is also opened on your machine
3. Get the frontend build tool. Run `cargo install trunk`.
4. Run the frontend. In this directory, run `trunk serve`. This will prompt for `sudo` password if database was started. Open it in browser at `http://localhost:8000`.
5. Open the ROS machine. `mkdir workspace` and `cd workspace`. Get the code: `mkdir src`, `cd src`, `git clone https://github.com/imaginary-units-pfur/pothole-ros-exporter`, change the server's IP address in `pothole_exporter/image_uploader.py` to match the backend from step 2. `cd ..`.
6. Get dependencies: `rosdep update`, `rosdep install -i --from-path src --rosdistro humble -y`. Build the package: `colcon build`.
7. Run the package: `source install/setup.sh`, `ros2 run pothole_ros_exporter uploader`.
8. Acquire data: run the robot, or `ros2 bag play`. 