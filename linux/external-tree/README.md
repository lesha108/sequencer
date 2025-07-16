## buildroot project customization (output file is image.itb, that combines compiled kernel, devicetree and rootfs)
Buildroot project allows to deliver ROOTFS file system and Linux kernel. For additional linux packages `_defconfig` files can be used. 
Also u-boot and ATF (for zynqmp) compilation are added
The structure of buildroot project:
```
external-tree/
|
+-- board/
|	|	
|	+-- zynq/
|	|	|
|	|	+-- rootfs-overlay/
|	|	+-- image.its
|	|	+-- kernel.config
|	|	+-- kernel.dts
|	|	+-- post-image.sh
|	|
|	+-- zynqmp/
|		|
|		+-- rootfs-overlay/
|		+-- image.its
|		+-- kernel.config
|		+-- kernel.dts
|		+-- post-image.sh
|	
|
+-- configs/
|	|
|	+-- zynq_defconfig
|	+-- zynqmp_defconfig
|	
|
+-- Config.in
+-- external.mk
+-- external.desc
```

### Steps are (or run build_zynq.sh / build_zynqmp.sh from the root)

* Clone buildroot repository `git clone https://github.com/buildroot/buildroot.git`.
* Checkout to version 2023.05 `git checkout tags/2023.05`
* Configure `buildroot` for zynq 7000 or zynqmp target

#### ZynqMP Ultrascale+

```
mkdir -p /path/to/build/dir/zynqmp
cd /path/to/build/dir/zynqmp
make --directory=/path/to/buildroot/git BR2_EXTERNAL=/path/to/this/git zynqmp_defconfig O=$PWD
```

#### Zynq 7000

```
mkdir -p /path/to/build/dir/zynq7k
cd /path/to/build/dir/zynq7k
make --directory=/path/to/buildroot/git BR2_EXTERNAL=/path/to/this/git zynq_defconfig O=$PWD
```
* Build Image

```
make
```
* Copy `image.itb` from `/build` directory to `/boot`

