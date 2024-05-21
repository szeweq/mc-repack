param($dir)

# Initialize an empty dictionary to store extension counts
$extensionCounts = @{}

# Example paths in format (<jar file name>: <entry path>) found for each extension
$exemplePaths = @{}

# Get all JAR files in the directory
Get-ChildItem -Path $dir -Filter "*.jar" | ForEach-Object {

  # Extract the JAR file path
  $jarPath = $_.FullName
  $jarName = $_.Name

  # Open the JAR file for reading using ZipFile
  $zipFile = [IO.Compression.ZipFile]::OpenRead($jarPath)

  # Loop through each entry in the ZIP file
  $zipFile.Entries | Where-Object { !$_.IsDirectory } | ForEach-Object {
    # Get the entry name and potential extension
    $entryName = $_.Name
    $entryPath = $_.FullName

    # Check if the entry name contains a dot (indicating extension)
    # Extract the extension (lowercase for case-insensitivity)
    $extension = $entryName.Split('.')[-1].ToLower()

    # Check if the extension is not empty
    if (($extension -ne "") -and ($extension -ne $entryName)) {
      # Add or increment the count for the extension in the dictionary
      if ($extensionCounts.ContainsKey($extension)) {
        $extensionCounts[$extension]++
      } else {
        $extensionCounts[$extension] = 1
        $exemplePaths[$extension] = $jarName + ": " + $entryPath
      }
    }
  }

  # Close the ZIP file (important for proper resource handling)
  $zipFile.Dispose()
}

$keysSorted = $extensionCounts.GetEnumerator() | Sort-Object -Property Value -Descending

# Display the extension counts
foreach ($ext in $keysSorted) {
  if ($ext.Value -gt 1) {
      Write-Host "$($ext.Key): $($ext.Value)"
    Write-Host "> Example: $($exemplePaths[$ext.Key])"
  }
}
