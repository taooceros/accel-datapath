add_rules("mode.debug", "mode.release", "mode.profile")

add_requires("libaccel-config", {system = true})
add_requires("fmt", {system = true})
add_requires("stdexec", {system = true})
add_requires("perfetto", {system = true})

set_strip("none")
set_symbols("debug")
add_ldflags("-fuse-ld=mold")

set_languages("c++23")
add_includedirs("include", "src")
add_links("stdc++exp")
add_cxxflags("-fno-omit-frame-pointer")

-- AddressSanitizer support: use `xmake f --policies=build.sanitizer.address`
option("asan")
    set_default(false)
    set_showmenu(true)
    set_description("Enable AddressSanitizer")
option_end()

if has_config("asan") then
    set_policy("build.sanitizer.address", true)
end

target("dsa-stdexec")
    set_kind("binary")
    add_files("src/**.cpp")
    add_packages("libaccel-config")
    add_packages("fmt")
    add_packages("stdexec")
    add_packages("perfetto")
    add_defines("DSA_ENABLE_TRACING")
    add_cflags("-menqcmd")
    add_cxxflags("-menqcmd")
    add_cflags("-movdir64b")
    add_cxxflags("-mmovdir64b")


target("dsa_benchmark")
    set_kind("binary")
    add_files("benchmark/dsa_benchmark.cpp")
    add_files("benchmark/benchmark_config.cpp")
    add_packages("libaccel-config")
    add_packages("fmt")
    add_packages("stdexec")
    add_cflags("-menqcmd")
    add_cxxflags("-menqcmd")
    add_cflags("-movdir64b")
    add_cxxflags("-mmovdir64b")
    add_files("src/dsa/dsa_instantiate.cpp")


target("dsa_benchmark_dynamic")
    set_kind("binary")
    add_files("benchmark/dsa_benchmark_dynamic.cpp")
    add_files("benchmark/benchmark_config.cpp")
    add_packages("libaccel-config")
    add_packages("fmt")
    add_packages("stdexec")
    add_cflags("-menqcmd")
    add_cxxflags("-menqcmd")
    add_cflags("-movdir64b")
    add_cxxflags("-mmovdir64b")
    add_files("src/dsa/dsa_instantiate.cpp")


target("dsa_launcher")
    set_kind("binary")
    set_languages("c11")
    add_files("tools/dsa_launcher.c")
    add_links("cap")
    after_build(function (target)
        os.exec("sudo setcap cap_sys_rawio+eip " .. target:targetdir() .. "/dsa_launcher")
    end)

target("task_queue_benchmark")
    set_kind("binary")
    add_files("benchmark/task_queue_benchmark.cpp")
    add_packages("fmt")
    add_packages("stdexec")

-- Examples
for _, name in ipairs({"data_move", "mem_fill", "compare", "compare_value", "dualcast", "crc_gen", "copy_crc", "cache_flush"}) do
    target("example_" .. name)
        set_kind("binary")
        add_files("examples/" .. name .. ".cpp")
        add_files("src/dsa/dsa_instantiate.cpp")
        add_packages("libaccel-config")
        add_packages("fmt")
        add_packages("stdexec")
        add_cflags("-menqcmd")
        add_cxxflags("-menqcmd")
        add_cflags("-movdir64b")
        add_cxxflags("-mmovdir64b")
end
