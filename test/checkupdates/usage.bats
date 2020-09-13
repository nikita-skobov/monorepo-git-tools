@test 'should not allow both --local and --remote at same time' {
    run $PROGRAM_PATH check-updates --local --remote repo_file.txt
    echo "$output"
    [[ $status != "0" ]]
    [[ "$output" == *"cannot be used with"* ]]
}


