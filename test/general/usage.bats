@test 'should show usage when using -h --help or help' {
    run $PROGRAM_PATH help
    echo "$output"
    [[ $output == *"USAGE"* ]]
    [[ $output == *"help"* ]]
    [[ $output == *"version"* ]]

    run $PROGRAM_PATH --help
    echo "$output"
    [[ $output == *"USAGE"* ]]
    [[ $output == *"help"* ]]
    [[ $output == *"version"* ]]

    run $PROGRAM_PATH -h
    echo "$output"
    [[ $output == *"USAGE"* ]]
    [[ $output == *"help"* ]]
    [[ $output == *"version"* ]]
}

@test 'should show current cargo.toml version' {
    cargo_txt=$(<./Cargo.toml)
    version_line=""

    last_ifs=$IFS
    IFS=$'\n'
    for line in $cargo_txt; do
        echo "line: $line"
        if [[ $line == *"version ="* ]]; then
            version_line="$line"
            break
        fi
    done
    IFS=$last_ifs

    version_str="$(echo $version_line | cut -d'"' -f 2)"

    run $PROGRAM_PATH --version
    [[ $output == *"$version_str"* ]]
}
