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
